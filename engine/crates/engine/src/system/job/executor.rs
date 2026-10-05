use super::fiber::{FiberHandle, FiberId};
use super::group::JobGroup;
use super::{Job, JobSystemShared};
use std::cell::RefCell;
use std::sync::Arc;
use std::thread::JoinHandle;
use windows_sys::Win32::System::Threading::SwitchToFiber;

thread_local! {
    static CURRENT_EXECUTOR: RefCell<Option<Executor>> =
        const { RefCell::new(None) };
}

// Fiber switches must happen outside this closure, after the TLS borrow ends.
fn with_current_executor<R>(f: impl FnOnce(&mut Executor) -> R) -> R {
    CURRENT_EXECUTOR.with(|slot| {
        let mut slot = slot.borrow_mut();
        let executor = slot.as_mut().expect("current thread has no Executor");
        f(executor)
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WorkerId(pub usize);
pub struct Worker {
    thread: JoinHandle<()>,
}
pub struct Executor {
    worker_id: WorkerId,          // worker thread id
    shared: Arc<JobSystemShared>, // thraed shared JobSystem

    manager_fiber: FiberHandle,
    current_fiber: Option<FiberId>,
}

impl Executor {
    pub fn new(
        worker_id: WorkerId,
        shared: Arc<JobSystemShared>,
        manager_fiber: FiberHandle,
    ) -> Self {
        Self {
            worker_id,
            shared,
            manager_fiber,
            current_fiber: None,
        }
    }

    // Call on the worker thread that owns manager_fiber, while on that fiber.
    pub fn run(self) {
        CURRENT_EXECUTOR.with(|slot| {
            let mut slot = slot.borrow_mut();
            assert!(slot.is_none(), "Executor already installed");
            *slot = Some(self);
        });

        loop {
            if with_current_executor(|executor| executor.shared.is_shutdown()) {
                break;
            }

            let next = with_current_executor(|executor| executor.next_ready_fiber());
            if let Some(fiber_id) = next {
                // Queue/state management must provide exclusive execution ownership.
                unsafe {
                    Self::resume_fiber(fiber_id);
                }
            } else {
                std::thread::yield_now();
            }
        }

        let executor = CURRENT_EXECUTOR
            .with(|slot| slot.borrow_mut().take())
            .expect("current thread has no Executor");
        unsafe {
            executor
                .manager_fiber
                .convert_back_to_thread()
                .expect("failed to convert manager fiber back to thread");
        }
    }

    /// # Safety
    /// Call only from the current job fiber. The caller must ensure the waiting
    /// fiber cannot be resumed by another worker before the switch completes.
    /// TODO: implement the parking/wakeup handshake in fiber state management.
    pub(crate) unsafe fn suspend_current_fiber(group: &JobGroup) {
        let (fiber_id, manager) = with_current_executor(|executor| {
            (
                executor.current_fiber.expect("no current fiber"),
                executor.manager_fiber.as_ptr(),
            )
        });

        if !group.add_waiter(fiber_id) {
            return;
        }

        unsafe {
            SwitchToFiber(manager);
        }
        // This job may now run on another worker. Do not reuse the old Executor.
    }

    // Only selects/assigns work. Never switches fibers while borrowing Executor.
    fn next_ready_fiber(&mut self) -> Option<FiberId> {
        if let Some(fiber_id) = self.shared.pop_ready_fiber() {
            return Some(fiber_id);
        }

        let Some(job) = self.shared.pop_ready_job() else {
            return None;
        };

        let Some(fiber_id) = self.shared.pop_free_fiber() else {
            self.shared.push_ready_job(job);
            return None;
        };

        self.assign_job(fiber_id, job);
        Some(fiber_id)
    }

    // Called on the current worker's manager fiber with exclusive ownership of
    // the target job fiber. TLS references and guards never cross the switch.
    unsafe fn resume_fiber(fiber_id: FiberId) {
        let target = with_current_executor(|executor| {
            assert!(
                executor.current_fiber.is_none(),
                "already running a job fiber"
            );
            let target = executor.shared.fiber_handle(fiber_id).as_ptr();
            executor.current_fiber = Some(fiber_id);
            target
        });

        unsafe {
            SwitchToFiber(target);
        }

        // Execution is back on this worker's manager fiber.
        with_current_executor(|executor| executor.current_fiber = None);
    }

    pub fn current_worker_id() -> WorkerId {
        with_current_executor(|executor| executor.worker_id)
    }

    pub fn current_job_fiber() -> Option<FiberId> {
        with_current_executor(|executor| executor.current_fiber)
    }

    pub(crate) fn current_shared() -> Arc<JobSystemShared> {
        with_current_executor(|executor| executor.shared.clone())
    }

    /// # Safety
    /// Call only from the current job fiber. Its execution ownership must not
    /// be published to another worker until the manager regains control.
    pub(crate) unsafe fn return_to_manager() {
        let manager = with_current_executor(|executor| {
            assert!(executor.current_fiber.is_some(), "not running a job fiber");
            executor.manager_fiber.as_ptr()
        });
        unsafe {
            SwitchToFiber(manager);
        }
        // A reused fiber may resume on another worker; consult TLS again.
    }

    fn assign_job(&mut self, fiber_id: FiberId, job: Job) {
        self.shared.assign_job(fiber_id, job);
    }

    pub fn current_fiber(&self) -> Option<FiberId> {
        self.current_fiber
    }

    pub fn worker_id(&self) -> WorkerId {
        self.worker_id
    }

    pub fn manager_fiber(&self) -> &FiberHandle {
        &self.manager_fiber
    }
}
