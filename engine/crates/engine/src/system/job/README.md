# Fiber job system

Windows-only stackful job execution. Engine Context integration is deliberately
outside this module.

```rust,ignore
let mut jobs = JobSystem::new(4, 16, 0)?; // workers, initial fibers, reserved stack bytes
let handle = jobs.handle();
let mut values = vec![0; 1024];

// Here the caller is outside the worker pool. Inside a job fiber, every live
// caller frame must also satisfy the stack-migration contract below.
unsafe {
    handle.scope(|scope| {
        let group = scope.group();
        for chunk in values.chunks_mut(128) {
            scope.spawn(&group, "job", move || chunk.fill(1)).unwrap();
        }
        scope.wait(&group).unwrap();
    })?;
}
jobs.shutdown()?;
```

- `spawn` accepts owned `Send + 'static` closures; `JobScope::spawn` accepts
  borrowed `Send` closures. Scope exit drains every scoped task, including nested
  submissions and body/job panics, before releasing borrowed data.
- `wait` seals its group. Submit all jobs in that group before waiting; use a new
  group for another batch. Groups belong to one JobSystem.
- `wait` and `scope` are **unsafe** because a suspended fiber can resume on another
  worker. All live data/references in the entire suspended call stack must allow
  execution and destruction there. Do not hold thread-affine locks/guards, TLS
  references, non-Send values, or other thread-bound state across suspension.
  A `Send` closure alone does not establish this property for its local variables.
  Switching during unwinding is rejected. Dependencies must be acyclic; a job
  must not wait on a group containing itself. Blocking OS calls still block a
  worker; `wait` is the cooperative fiber suspension mechanism.
- The pool grows when waiting parents consume the initial stacks. Allocation
  failure fails the selected job and releases its counters; that job's captures
  are dropped without executing its body. This avoids deadlock caused solely by
  a fixed fiber capacity, but does not repair cyclic dependencies. Extra stacks
  stay allocated until shutdown, so deeply nested waits can use substantial memory.
- `shutdown` rejects new submissions (including child submissions), drains
  accepted work, joins workers, and deletes the fiber pool. Submitters must handle
  the returned error during shutdown. Handles may survive shutdown but cannot
  submit new work. Call shutdown/drop from outside its own workers.
- Job failures are retained in their group; `wait` reports the failure count.
  A scoped child failure also makes scope return an error. A body panic is resumed
  after scoped jobs have drained. Cancellation of running jobs is not supported.

## Execution history

Both handle and scope submissions require a name: `spawn(&group, "name", task)`.
Each job gets a unique per-system `JobId`. Names and IDs are shared through
`Arc<JobInfo>`; no global name registry is retained.

```rust,ignore
let handle = jobs.handle();
let profiler = handle.profiler();
for index in 0..profiler.worker_count() {
    let history = profiler.worker_history(index).unwrap();
    for segment in history.segments() {
        println!("{:?} {} {:?}..{:?}", segment.job.id, segment.job.name,
            segment.started_at, segment.ended_at);
    }
}
```

Each worker retains its latest 4,096 completed dispatch intervals. Older entries
are evicted and counted by `dropped_segments()`. `worker_history()` copies only
one bounded history, allowing the caller to release the worker lock immediately.
Replace old copies instead of accumulating them indefinitely.

Times share one monotonic origin and include fiber-switch overhead and any OS
descheduling while dispatched. They are not OS CPU utilization measurements.
Suspended intervals are excluded; an ongoing dispatch appears only after it
returns to the manager. Migrated jobs keep their ID/name across workers, and
recycled fibers get the newly assigned job's metadata. Recording is currently
always enabled and adds overhead to each dispatch.

## Tests

Run tests for this module independently of engine dependencies:

```powershell
cargo test --manifest-path crates/engine/src/system/job/test_harness/Cargo.toml --offline
```

The harness compiles the actual module. Runtime tests cover waiting with a single
initial fiber, cross-worker resume, borrowed chunks, nested scopes, panic cleanup,
group ownership, and shutdown. Compile-fail tests cover scope/data escaping.


# コンパイラ最適化によるバグ 2026/11/24
JobSystemではfiberを採用している。Workerは
* Worker
  - Executor: `CURRENT_EXECUTOR` スレッドローカル変数
  - Manager Fiber: Jobを実行するFiberを管理するスケジューラー
  - Fiber: スタックを持つ、ジョブを実行する単位

WorkerスレッドはExecutorをTLS (Thread Loacal Storage) に保持する。例えば、WorkerAはアドレス0x1000にExecutorを持つ、WorkerBはアドレス0x2000にExecutorを持つ。

各Executorは`manger_fiber`と`current_fiber`があり、実行するfiberを切り替えることで実行する処理単位を切り替える。

* `manager_fiber`: workerに固定されている。ジョブの選択・実行後の管理を行うfiber
* `current_fiber`: 現在実行しているjobのfiber ID。実行していない場合は`None`になる

流れは、
1. Workerスレッド (WorkerA) が`manager_fiber`を動かし、共有キューの`JobSystemShared`から`next_ready_fiber()`で実行可能jobを未使用fiberに登録・取得
2. `resume_fiber(id)`呼び出す
  - `executor.current_fiber=Some(id)`で、Executorの`current_fiber`に取得したfiberを割り当てる。
  - `resume_fiber(id)`内で`SwitchToFiber(id)`が呼ばれる
    - `job_fiber_entry()`が呼ばれ、処理が開始・再開される
    - もしjobがpending状態 (他の処理単位を待つ状態) になったらfiberはスタックを保持したまま、中断する
    - 待機fiberは`JobSystemShared`の共通キューにpush
  - WorkerAの`return_to_manager()`を呼びだす
    - `assert!(executor.current_fiber.is_some())`
  - `executor.executor=None`
3. 他のWorkerスレッド (WorkerB) がpendingファイバーを`naxt_ready_fiber()`で取得する
4. WorkerBが`resume_fiber(pending)`でpendingファイバーを再開
  - `return_to_manager()`を呼びだす
    - `assert!(executor.current_fiber.is_some())`
  - `executor.executor=None`
5. WorkerBの`current_fiber=None`にする


```rust
pub(crate) struct Executor {
    worker_id: WorkerId,
    shared: Arc<JobSystemShared>,
    // Thread-affine: created and destroyed only within this worker.
    manager_fiber: FiberHandle,
    current_fiber: Option<FiberId>,
}
```
```rust
thread_local! {
    static CURRENT_EXECUTOR: RefCell<Option<Executor>>
        = const { RefCell::new(None) };
}
```
```rust
impl Executor {
    fn run() -> Result<()> {
        loop {
            let shared = Self::current_shared();
            if shared.is_shutdown() {
                break;
            }
            shared.process_wake_requests();
            if let Some(id) = shared.next_ready_fiber() {
                unsafe {
                    Self::resume_fiber(id);
                }
            } else {
                shared.wait_for_work();
            }
        }
        let executor = CURRENT_EXECUTOR
            .with(|slot| slot.borrow_mut().take())
            .unwrap();
        unsafe { executor.manager_fiber.convert_back_to_thread() }
    }

    ...

    pub(crate) unsafe fn return_to_manager() {
        let manager = with_current_executor(|executor| {
            assert!(executor.current_fiber.is_some());
            executor.manager_fiber.as_ptr()
        });
        unsafe {
            SwitchToFiber(manager);
        }
    }
}
```

```rust
fn with_current_executor<R>(f: impl FnOnce(&mut Executor) -> R) -> R {
    CURRENT_EXECUTOR.with(|slot| f(slot.borrow_mut().as_mut().expect("no current Executor")))
}
```

```rust
pub(crate) unsafe extern "system" fn job_fiber_entry(_parameter: *mut c_void) {
    let id = Executor::current_job_fiber().expect("job fiber has no FiberId");
    loop {
        let shared = Executor::current_shared();
        // Assigned -> Running
        let job = shared.fiber(id).take_job();
        let completed = job.execute();
        shared.fiber(id).mark_finished(completed);
        // Do not keep this Arc on a suspended/reusable fiber's stack: that
        // would retain the shared pool until this fiber executes again.
        drop(shared);

        unsafe {
            Executor::return_to_manager();
        }
    }
}
```


異なるWorkerスレッド (WorkerA, WorkerB) が同じfiberを処理することがある。この場合、コンパイラの最適化によって、想定しないことが起こる。

想定している動作は以下のようになる
* WorkerA
  - fiber1開始
    - `workerA.executor.current_fiber=fiber1`
  - fiber1一時停止
    - `assert!(workerA.executor.current_fiber.is_some())`
    - `workerA.executor.current_fiber=None`
* WorkerB
  - fiber1再開
    - `workerB.executor.current_fiber=fiber1`
  - fiber1終了
    - `assert!(workerB.executor.current_fiber.is_some())`
    - `workerB.executor.current_fiber=None`

Executorがfiberを`manager_fiber`に切り替えるタイミングで毎回`assert!(current_fiber.is_some())`で`current_fiber`がNoneでないことをチェックする。

WorkerがExecutor取得するとき`with_current_executor()`を呼ぶ。この関数は現在のWorkerのExecutorを一時的に借りて、クロージャ`f`を実行する。

コンパイラによる最適化によるThin LTO(Thin Link Time Optimization) により、上記の2つの部分で問題が生じる。





