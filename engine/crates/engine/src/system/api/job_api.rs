use crate::JobRunSegment;
use crate::system::job::JobSystemHandle;
use anyhow::{Result, anyhow};
use std::cell::RefCell;
use std::collections::VecDeque;
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

    /// Returns retained, completed execution segments for a worker, oldest first.
    /// The returned collection is independent of the live worker history.
    fn worker_history(&self, id: usize) -> Result<VecDeque<JobRunSegment>> {
        let system = self.job_system()?;
        let history = system
            .profiler()
            .worker_history(id)
            .ok_or_else(|| anyhow!("unknown worker id: {}", id))?;
        Ok(history.into_segments())
    }

    /// Searches retained segments on all workers by exact job name.
    /// Results are sorted by start time; no matches returns an empty Vec.
    fn get_job_segments_from_name(&self, name: &str) -> Result<Vec<JobRunSegment>> {
        let system = self.job_system()?;
        Ok(system.profiler().get_job_segments_from_name(name))
    }

    fn worker_count(&self) -> Result<usize> {
        let system = self.job_system()?;
        Ok(system.profiler().worker_count())
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
    fn history_api_returns_owned_segments_and_filters_names() {
        let probe = Probe;
        assert!(probe.worker_history(0).is_err());
        assert!(probe.worker_count().is_err());
        assert!(probe.get_job_segments_from_name("target").is_err());

        let system = JobSystem::new(2, 2, 0).unwrap();
        let jobs = system.handle();
        let _guard = register_job_system(jobs.clone());
        assert_eq!(probe.worker_count().unwrap(), 2);
        assert!(probe.worker_history(0).unwrap().is_empty());
        assert!(probe.worker_history(2).is_err());

        let group = JobGroup::new();
        for name in ["target", "other", "target"] {
            jobs.spawn(&group, name, || {}).unwrap();
        }
        unsafe {
            jobs.wait(&group).unwrap();
        }

        let mut count = 0;
        for id in 0..probe.worker_count().unwrap() {
            let mut segments = probe.worker_history(id).unwrap();
            assert!(segments.iter().all(|segment| segment.worker_id.0 == id));
            count += segments.len();
            segments.clear();
        }
        assert_eq!(count, 3);
        let matching = probe.get_job_segments_from_name("target").unwrap();
        assert_eq!(matching.len(), 2);
        assert!(
            matching
                .iter()
                .all(|segment| segment.job.name.as_ref() == "target")
        );
        assert_ne!(matching[0].job.id, matching[1].job.id);
        assert!(matching[0].started_at <= matching[1].started_at);
        assert_eq!(probe.get_job_segments_from_name("other").unwrap().len(), 1);
        assert!(
            probe
                .get_job_segments_from_name("missing")
                .unwrap()
                .is_empty()
        );
        assert_eq!(probe.get_job_segments_from_name("target").unwrap().len(), 2);
    }

    #[test]
    fn registration_is_thread_local_and_restores_after_nested_panic() {
        let probe = Probe;
        assert!(probe.job_system().is_err());
        let first = JobSystem::new(1, 1, 0).unwrap();
        let second = JobSystem::new(1, 1, 0).unwrap();
        let group = JobGroup::new();
        {
            let _outer = register_job_system(first.handle());
            probe
                .job_system()
                .unwrap()
                .spawn(&group, "job", || {})
                .unwrap();
            std::thread::spawn(|| assert!(Probe.job_system().is_err()))
                .join()
                .unwrap();
            let result = catch_unwind(AssertUnwindSafe(|| {
                let _inner = register_job_system(second.handle());
                // Group ownership verifies that this is the second system.
                assert!(
                    probe
                        .job_system()
                        .unwrap()
                        .spawn(&group, "job", || {})
                        .is_err()
                );
                panic!("nested stage failed");
            }));
            assert!(result.is_err());
            // The outer system is restored even after unwinding.
            probe
                .job_system()
                .unwrap()
                .spawn(&group, "job", || {})
                .unwrap();
            unsafe {
                probe.job_system().unwrap().wait(&group).unwrap();
            }
        }
        assert!(probe.job_system().is_err());
    }
}
