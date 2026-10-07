use super::JobSystemShared;
use super::fiber::{FiberHandle, FiberId};
use super::group::JobGroup;
use anyhow::{Result, anyhow};
use std::cell::RefCell;
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use windows_sys::Win32::System::Threading::SwitchToFiber;

thread_local! {
    static CURRENT_EXECUTOR: RefCell<Option<Executor>> = const { RefCell::new(None) };
}
// The TLS borrow must end before any fiber switch.
fn with_current_executor<R>(f: impl FnOnce(&mut Executor) -> R) -> R {
    CURRENT_EXECUTOR.with(|slot| f(slot.borrow_mut().as_mut().expect("no current Executor")))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct WorkerId(pub usize);
pub(crate) struct Worker {
    thread: JoinHandle<Result<()>>,
}
pub(crate) struct Executor {
    worker_id: WorkerId,
    shared: Arc<JobSystemShared>,
    // Thread-affine: created and destroyed only within this worker.
    manager_fiber: FiberHandle,
    current_fiber: Option<FiberId>,
}
impl Worker {
    pub(crate) fn start(id: WorkerId, shared: Arc<JobSystemShared>) -> Result<Self> {
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name(format!("job-worker-{}", id.0))
            .spawn(move || -> Result<()> {
                let manager = match unsafe { FiberHandle::manager_fiber_from_thread() } {
                    Ok(manager) => manager,
                    Err(error) => {
                        let _ = ready_tx.send(Err(error.to_string()));
                        return Err(error);
                    }
                };
                let executor = Executor {
                    worker_id: id,
                    shared,
                    manager_fiber: manager,
                    current_fiber: None,
                };
                // Install TLS before acknowledging successful startup.
                CURRENT_EXECUTOR.with(|slot| {
                    assert!(slot.borrow().is_none());
                    *slot.borrow_mut() = Some(executor);
                });
                let _ = ready_tx.send(Ok(()));
                Executor::run()
            })?;
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self { thread }),
            startup => {
                let _ = thread.join();
                Err(anyhow!("worker startup failed: {startup:?}"))
            }
        }
    }
    pub(crate) fn join(self) -> Result<()> {
        self.thread.join().map_err(|_| anyhow!("worker panicked"))?
    }
}
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
    /// # Safety
    /// Caller must satisfy the complete-stack migration requirements of wait.
    pub(crate) unsafe fn suspend_current_fiber(group: &JobGroup) {
        let (id, shared, manager) = with_current_executor(|executor| {
            (
                executor.current_fiber.expect("wait outside job fiber"),
                executor.shared.clone(),
                executor.manager_fiber.as_ptr(),
            )
        });
        shared.fiber(id).begin_parking();
        if !group.add_waiter(id) {
            shared.fiber(id).cancel_parking();
            return;
        }
        drop(shared);
        unsafe {
            SwitchToFiber(manager);
        }
        // Subsequent TLS access resolves the worker on which we resumed.
    }
    unsafe fn resume_fiber(id: FiberId) {
        let target = with_current_executor(|executor| {
            assert!(executor.current_fiber.is_none());
            let fiber = executor.shared.fiber(id);
            debug_assert_eq!(fiber.id(), id);
            fiber.prepare_resume();
            executor.current_fiber = Some(id);
            fiber.handle().as_ptr()
        });
        unsafe {
            // return to target fiber
            SwitchToFiber(target);
        }

        // manager fiber
        let shared = with_current_executor(|executor| {
            executor.current_fiber = None;
            executor.shared.clone()
        });
        let fiber = shared.fiber(id);
        if let Some(ready) = fiber.finish_parking() {
            if ready {
                shared.push_ready_fiber(id);
            }
            // A Waiting fiber may already be owned by another worker now.
            return;
        }
        let completed = fiber
            .take_completed()
            .expect("fiber returned without parking/completion");
        fiber.recycle();
        shared.push_free_fiber(id);
        // Release outstanding only AFTER the original fiber became reusable.
        shared.complete(completed);
    }
    pub(crate) fn current_worker_id() -> WorkerId {
        with_current_executor(|executor| executor.worker_id)
    }
    pub(crate) fn current_job_fiber() -> Option<FiberId> {
        with_current_executor(|executor| executor.current_fiber)
    }
    pub(crate) fn current_shared() -> Arc<JobSystemShared> {
        with_current_executor(|executor| executor.shared.clone())
    }
    pub(crate) fn try_current_shared() -> Option<Arc<JobSystemShared>> {
        CURRENT_EXECUTOR.with(|slot| {
            slot.borrow()
                .as_ref()
                .map(|executor| executor.shared.clone())
        })
    }
    /// # Safety
    /// Only the current job fiber may call this. Do not publish its execution
    /// ownership until the manager has regained control.
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
