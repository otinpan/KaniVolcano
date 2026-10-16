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
