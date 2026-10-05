use super::JobSystemHandle;
use super::group::JobGroup;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub type JobTask = Box<dyn FnOnce() + Send + 'static>;

pub struct Job {
    task: JobTask,
    group: JobGroup,
}

// Consumed by the manager after the job fiber has switched back to it.
pub(crate) struct CompletedJob {
    pub(crate) group: JobGroup,
    pub(crate) result: std::thread::Result<()>,
}

impl Job {
    pub(crate) fn execute(self) -> CompletedJob {
        let Self { task, group } = self;
        CompletedJob {
            group,
            result: catch_unwind(AssertUnwindSafe(task)),
        }
    }
}
pub struct JobScope<'scope> {
    jobs: JobSystemHandle,
    scope_group: JobGroup,
    _marker: std::marker::PhantomData<&'scope mut ()>,
}

impl<'scope> JobScope<'scope> {
    pub fn group(&self) -> JobGroup {
        JobGroup::new()
    }

    pub fn spawn<F>(&self, group: &JobGroup, task: F)
    where
        F: FnOnce() + Send + 'scope,
    {
        group.add();
        self.scope_group.add();

        self.jobs
            .push_job(Job::new(task, group.clone(), self.scope_group.clone()));
    }

    pub fn wait(&self, group: &JobGroup) {
        self.jobs.wait(group);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[test]
    fn execution_runs_once_and_leaves_completion_to_manager() {
        let count = Arc::new(AtomicUsize::new(0));
        let job_count = count.clone();
        let group = JobGroup::new();
        group.add();
        let job = Job {
            task: Box::new(move || {
                job_count.fetch_add(1, Ordering::Relaxed);
            }),
            group: group.clone(),
        };

        let completed = job.execute();
        assert!(completed.result.is_ok());
        assert_eq!(count.load(Ordering::Relaxed), 1);
        assert_eq!(group.pending(), 1);
        completed.group.finish();
        assert!(group.is_finished());
    }

    #[test]
    fn execution_captures_panic_and_preserves_group() {
        let group = JobGroup::new();
        group.add();
        let job = Job {
            task: Box::new(|| panic!("job failed")),
            group: group.clone(),
        };

        let completed = job.execute();
        let panic = completed.result.unwrap_err();
        assert_eq!(panic.downcast_ref::<&str>(), Some(&"job failed"));
        assert_eq!(group.pending(), 1);
        completed.group.finish();
        assert!(group.is_finished());
    }

    #[test]
    fn execution_releases_task_captures_before_completion() {
        let value = Arc::new(());
        let weak = Arc::downgrade(&value);
        let job = Job {
            task: Box::new(move || {
                assert_eq!(Arc::strong_count(&value), 1);
            }),
            group: JobGroup::new(),
        };

        let completed = job.execute();
        assert!(completed.result.is_ok());
        assert!(weak.upgrade().is_none());
    }
}
