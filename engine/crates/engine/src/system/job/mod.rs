mod executor;
mod fiber;
mod group;
mod job;

pub use executor::{Worker, WorkerId};
pub use fiber::{Fiber, FiberHandle, FiberId};
pub use group::JobGroup;
pub use job::{Job, JobTask};
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, atomic::AtomicBool};

// use from engine
#[derive(Clone)]
pub struct JobSystemHandle {
    shared: Arc<JobSystemShared>,
}

impl JobSystemHandle {
    // The caller increments the job's group counters before publishing it.
    pub(crate) fn push_job(&self, job: Job) {
        self.shared.push_ready_job(job);
    }
}

pub struct JobSystem {
    shared: Arc<JobSystemShared>,
    workers: Vec<Worker>,
}

pub struct JobSystemShared {
    fibers: Vec<Box<Fiber>>,
    ready_jobs: Mutex<VecDeque<Job>>,
    ready_fibers: Mutex<VecDeque<FiberId>>,
    free_fibers: Mutex<VecDeque<FiberId>>,

    wake: Condvar,

    shutdown: AtomicBool,
}

impl JobSystem {}

impl JobSystemShared {
    pub fn new() -> Self {
        Self {
            fibers: Vec::new(),

            ready_jobs: Mutex::new(VecDeque::new()),
            ready_fibers: Mutex::new(VecDeque::new()),
            free_fibers: Mutex::new(VecDeque::new()),

            wake: Condvar::new(),

            shutdown: AtomicBool::new(false),
        }
    }

    // Initialize before putting Shared in an Arc and starting workers.
    // IDs and handles remain stable for the lifetime of the pool.
    pub(crate) fn init_fiber_pool(
        &mut self,
        fiber_count: usize,
        stack_size: usize,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(fiber_count > 0, "fiber_count must be greater than zero");
        anyhow::ensure!(self.fibers.is_empty(), "fiber pool already initialized");
        anyhow::ensure!(
            self.ready_jobs.get_mut().unwrap().is_empty(),
            "initialize fiber pool before submitting jobs"
        );
        anyhow::ensure!(
            self.free_fibers.get_mut().unwrap().is_empty()
                && self.ready_fibers.get_mut().unwrap().is_empty(),
            "initialize fiber pool before publishing fiber IDs"
        );

        let mut handles = Vec::with_capacity(fiber_count);
        for _ in 0..fiber_count {
            // Newly created fibers have not run yet. Their entry obtains its
            // FiberId from worker TLS, set before the first SwitchToFiber.
            let created = unsafe {
                FiberHandle::create(stack_size, fiber::job_fiber_entry, std::ptr::null_mut())
            };
            match created {
                Ok(handle) => handles.push(handle),
                Err(error) => {
                    // FiberHandle has no automatic Drop. Release the partial
                    // pool; none of these fibers has ever been executed.
                    for handle in handles {
                        unsafe {
                            handle.delete();
                        }
                    }
                    return Err(error);
                }
            }
        }

        self.fibers = handles
            .into_iter()
            .enumerate()
            .map(|(index, handle)| Box::new(Fiber::new(FiberId(index), handle)))
            .collect();
        self.free_fibers
            .get_mut()
            .unwrap()
            .extend((0..fiber_count).map(FiberId));
        Ok(())
    }

    pub(crate) fn push_ready_job(&self, job: Job) {
        self.ready_jobs.lock().unwrap().push_back(job);
        self.wake.notify_one();
    }

    pub(crate) fn pop_ready_job(&self) -> Option<Job> {
        self.ready_jobs.lock().unwrap().pop_front()
    }

    pub(crate) fn pop_ready_fiber(&self) -> Option<FiberId> {
        self.ready_fibers.lock().unwrap().pop_front()
    }

    pub(crate) fn pop_free_fiber(&self) -> Option<FiberId> {
        self.free_fibers.lock().unwrap().pop_front()
    }

    pub(crate) fn is_shutdown(&self) -> bool {
        self.shutdown.load(std::sync::atomic::Ordering::Acquire)
    }

    pub(crate) fn fiber_handle(&self, fiber_id: FiberId) -> &FiberHandle {
        self.fibers[fiber_id.0].handle()
    }

    pub(crate) fn assign_job(&self, fiber_id: FiberId, job: Job) {
        self.fibers[fiber_id.0].assign_job(job);
    }

    pub(crate) fn fiber(&self, fiber_id: FiberId) -> &Fiber {
        &self.fibers[fiber_id.0]
    }
}
