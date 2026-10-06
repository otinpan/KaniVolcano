use super::group::JobGroup;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub(crate) type JobTask = Box<dyn FnOnce() + Send + 'static>;
pub(crate) struct Job {
    task: JobTask,
    pub(crate) group: JobGroup,
    pub(crate) scope_group: Option<JobGroup>,
}
pub(crate) struct CompletedJob {
    pub(crate) group: JobGroup,
    pub(crate) scope_group: Option<JobGroup>,
    pub(crate) result: std::thread::Result<()>,
}
impl Job {
    pub(crate) fn new_owned<F>(task: F, group: JobGroup) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Self {
            task: Box::new(task),
            group,
            scope_group: None,
        }
    }
    pub(crate) fn new_scoped(task: JobTask, group: JobGroup, scope_group: JobGroup) -> Self {
        Self {
            task,
            group,
            scope_group: Some(scope_group),
        }
    }
    pub(crate) fn execute(self) -> CompletedJob {
        let Self {
            task,
            group,
            scope_group,
        } = self;
        CompletedJob {
            group,
            scope_group,
            result: catch_unwind(AssertUnwindSafe(task)),
        }
    }
    pub(crate) fn cancel(self, reason: String) -> CompletedJob {
        let Self {
            task,
            group,
            scope_group,
        } = self;
        // Destroy captures before publishing either completion counter.
        let result = catch_unwind(AssertUnwindSafe(|| drop(task)));
        CompletedJob {
            group,
            scope_group,
            result: match result {
                Err(error) => Err(error),
                Ok(()) => Err(Box::new(reason)),
            },
        }
    }
}
