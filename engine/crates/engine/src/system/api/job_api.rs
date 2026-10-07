use crate::system::job::JobSystemHandle;
use anyhow::{Result, anyhow};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

thread_local! {
    static CURRENT_JOBS: RefCell<Option<JobSystemHandle>> =
        const { RefCell::new(None) };
}

// A registration belongs to the thread that created it. Rc makes this guard
// !Send/!Sync, so it cannot restore another thread's TLS by accident.
#[must_use = "keep the guard alive while systems use JobApi"]
pub(crate) struct JobSystemGuard {
    previous: Option<JobSystemHandle>,
    _thread_bound: PhantomData<Rc<()>>,
}

pub(crate) fn register_job_system(handle: JobSystemHandle) -> JobSystemGuard {
    let previous = CURRENT_JOBS.with(|slot| slot.replace(Some(handle)));
    JobSystemGuard {
        previous,
        _thread_bound: PhantomData,
    }
}

impl Drop for JobSystemGuard {
    fn drop(&mut self) {
        CURRENT_JOBS.with(|slot| {
            slot.replace(self.previous.take());
        });
    }
}

pub trait JobApi {
    fn job_system(&self) -> Result<JobSystemHandle> {
        CURRENT_JOBS.with(|slot| {
            slot.borrow()
                .clone()
                .ok_or_else(|| anyhow!("JobSystem is not registered"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::job::{JobGroup, JobSystem};
    use std::panic::{AssertUnwindSafe, catch_unwind};

    struct Probe;
    impl JobApi for Probe {}

    #[test]
    fn registration_is_thread_local_and_restores_after_nested_panic() {
        let probe = Probe;
        assert!(probe.job_system().is_err());
        let first = JobSystem::new(1, 1, 0).unwrap();
        let second = JobSystem::new(1, 1, 0).unwrap();
        let group = JobGroup::new();
        {
            let _outer = register_job_system(first.handle());
            probe.job_system().unwrap().spawn(&group, || {}).unwrap();
            std::thread::spawn(|| assert!(Probe.job_system().is_err()))
                .join()
                .unwrap();
            let result = catch_unwind(AssertUnwindSafe(|| {
                let _inner = register_job_system(second.handle());
                // Group ownership verifies that this is the second system.
                assert!(probe.job_system().unwrap().spawn(&group, || {}).is_err());
                panic!("nested stage failed");
            }));
            assert!(result.is_err());
            // The outer system is restored even after unwinding.
            probe.job_system().unwrap().spawn(&group, || {}).unwrap();
            unsafe {
                probe.job_system().unwrap().wait(&group).unwrap();
            }
        }
        assert!(probe.job_system().is_err());
    }
}
