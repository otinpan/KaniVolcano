mod executor;
mod fiber;
mod group;
mod job;

use anyhow::{Result, ensure};
use executor::{Executor, Worker, WorkerId};
use fiber::{Fiber, FiberHandle, FiberId};
pub use group::JobGroup;
use job::{CompletedJob, Job};
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

static NEXT_SYSTEM_ID: AtomicUsize = AtomicUsize::new(1);

pub struct JobScope<'scope, 'env: 'scope> {
    jobs: JobSystemHandle,
    scope_group: JobGroup,
    _scope: PhantomData<&'scope mut &'scope ()>,
    _env: PhantomData<&'env mut &'env ()>,
}

impl<'scope, 'env> JobScope<'scope, 'env> {
    pub fn group(&self) -> JobGroup {
        JobGroup::new()
    }
    pub fn spawn<F>(&'scope self, group: &JobGroup, task: F) -> Result<()>
    where
        F: FnOnce() + Send + 'scope,
    {
        let task: Box<dyn FnOnce() + Send + 'scope> = Box::new(task);
        // HRTB prevents this scope escaping. Its counter is released only after
        // the task and all borrowed captures have been destroyed, also on panic.
        let task =
            unsafe { std::mem::transmute::<Box<dyn FnOnce() + Send + 'scope>, job::JobTask>(task) };
        self.jobs.submit(Job::new_scoped(
            task,
            group.clone(),
            self.scope_group.clone(),
        ))
    }
    /// # Safety
    /// The complete suspended stack must support migration; see handle.wait.
    pub unsafe fn wait(&self, group: &JobGroup) -> Result<()> {
        unsafe { self.jobs.wait(group) }
    }
}

#[derive(Clone)]
pub struct JobSystemHandle {
    shared: Arc<JobSystemShared>,
}

impl JobSystemHandle {
    pub fn spawn<F>(&self, group: &JobGroup, task: F) -> Result<()>
    where
        F: FnOnce() + Send + 'static,
    {
        self.submit(Job::new_owned(task, group.clone()))
    }
    fn submit(&self, job: Job) -> Result<()> {
        let mut state = self.shared.scheduler.lock().unwrap();
        ensure!(state.accepting, "JobSystem is shutting down");
        job.group.add(self.shared.id)?;
        if let Some(scope) = &job.scope_group {
            if let Err(error) = scope.add(self.shared.id) {
                state.wakes.extend(job.group.finish(Ok(())));
                self.shared.wake.notify_all();
                return Err(error);
            }
        }
        state.outstanding += 1;
        state.jobs.push_back(job);
        self.shared.wake.notify_all();
        Ok(())
    }
    /// Seals the group against subsequent submissions.
    /// External callers block; job fibers suspend and release their worker.
    /// # Safety
    /// On a job fiber, ALL live values/references on its entire stack, including
    /// caller frames, must allow execution/destruction on another worker.
    /// No thread-affine guards, TLS references, or non-Send values may cross wait.
    /// Dependencies must be acyclic; a job must not wait for its own group.
    pub unsafe fn wait(&self, group: &JobGroup) -> Result<()> {
        ensure!(
            !std::thread::panicking(),
            "cannot switch fibers during unwinding"
        );
        if let Some(current) = Executor::try_current_shared() {
            ensure!(
                Arc::ptr_eq(&current, &self.shared),
                "wait from another JobSystem"
            );
        }
        group.seal(self.shared.id)?;
        unsafe {
            self.wait_internal(group)?;
        }
        group.check_result()
    }
    unsafe fn wait_internal(&self, group: &JobGroup) -> Result<()> {
        if let Some(current) = Executor::try_current_shared() {
            ensure!(
                Arc::ptr_eq(&current, &self.shared),
                "wait from another JobSystem"
            );
            drop(current);
            unsafe {
                Executor::suspend_current_fiber(group);
            }
        } else {
            group.wait_blocking();
        }
        Ok(())
    }
    /// Borrowed jobs finish before return, including when the body panics.
    /// # Safety
    /// Scope exit on a job fiber can suspend/migrate the entire caller stack.
    /// All requirements of wait apply, including during panic cleanup.
    pub unsafe fn scope<'env, F, R>(&self, f: F) -> Result<R>
    where
        F: for<'scope> FnOnce(&'scope JobScope<'scope, 'env>) -> R,
    {
        ensure!(
            !std::thread::panicking(),
            "cannot enter a job scope during unwinding"
        );
        if let Some(current) = Executor::try_current_shared() {
            ensure!(
                Arc::ptr_eq(&current, &self.shared),
                "scope from another JobSystem"
            );
        }
        let scope = JobScope {
            jobs: self.clone(),
            scope_group: JobGroup::new(),
            _scope: PhantomData,
            _env: PhantomData,
        };
        let result = catch_unwind(AssertUnwindSafe(|| f(&scope)));
        // Keep this internal counter open: counted children can spawn descendants.
        unsafe {
            self.wait_internal(&scope.scope_group)?;
        }
        match result {
            Ok(value) => {
                scope.scope_group.check_result()?;
                Ok(value)
            }
            Err(panic) => resume_unwind(panic),
        }
    }
}

pub struct JobSystem {
    shared: Arc<JobSystemShared>,
    workers: Vec<Worker>,
}
impl JobSystem {
    pub fn new(worker_count: usize, fiber_count: usize, stack_size: usize) -> Result<Self> {
        ensure!(
            worker_count > 0 && fiber_count > 0,
            "worker/fiber counts must be positive"
        );
        let shared = Arc::new(JobSystemShared::new(stack_size));
        for _ in 0..fiber_count {
            shared.allocate_fiber()?;
        }
        let mut system = Self {
            shared,
            workers: Vec::new(),
        };
        for index in 0..worker_count {
            system
                .workers
                .push(Worker::start(WorkerId(index), system.shared.clone())?);
        }
        Ok(system)
    }
    pub fn handle(&self) -> JobSystemHandle {
        JobSystemHandle {
            shared: self.shared.clone(),
        }
    }
    /// Drain outstanding work and join workers. Call outside this system.
    pub fn shutdown(&mut self) -> Result<()> {
        if let Some(current) = Executor::try_current_shared() {
            ensure!(
                !Arc::ptr_eq(&current, &self.shared),
                "cannot join current Worker"
            );
        }
        let mut state = self.shared.scheduler.lock().unwrap();
        state.accepting = false;
        while state.outstanding != 0 {
            state = self.shared.wake.wait(state).unwrap();
        }
        self.shared.shutdown.store(true, Ordering::Release);
        self.shared.wake.notify_all();
        drop(state);
        let mut error = None;
        for worker in self.workers.drain(..) {
            if let Err(e) = worker.join() {
                error.get_or_insert(e);
            }
        }
        if let Some(error) = error {
            return Err(error);
        }
        // Handles may outlive JobSystem; reclaim stacks at shutdown anyway.
        self.shared.fibers.lock().unwrap().clear();
        self.shared.scheduler.lock().unwrap().free.clear();
        Ok(())
    }
}
impl Drop for JobSystem {
    fn drop(&mut self) {
        // Self-join or a broken worker cannot safely reclaim suspended stacks.
        if self.shutdown().is_err() {
            std::process::abort();
        }
    }
}
#[derive(Default)]
struct Scheduler {
    jobs: VecDeque<Job>,
    ready: VecDeque<FiberId>,
    free: VecDeque<FiberId>,
    wakes: VecDeque<FiberId>,
    outstanding: usize,
    accepting: bool,
}
pub(crate) struct JobSystemShared {
    id: usize,
    fibers: Mutex<Vec<Arc<Fiber>>>,
    scheduler: Mutex<Scheduler>,
    wake: Condvar,
    shutdown: AtomicBool,
    stack_size: usize,
}
impl JobSystemShared {
    fn new(stack_size: usize) -> Self {
        let id = NEXT_SYSTEM_ID
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .expect("JobSystem ID exhausted");
        Self {
            id,
            fibers: Mutex::new(Vec::new()),
            scheduler: Mutex::new(Scheduler {
                accepting: true,
                ..Scheduler::default()
            }),
            wake: Condvar::new(),
            shutdown: AtomicBool::new(false),
            stack_size,
        }
    }
    fn allocate_fiber(&self) -> Result<FiberId> {
        let handle = unsafe {
            FiberHandle::create(
                self.stack_size,
                fiber::job_fiber_entry,
                std::ptr::null_mut(),
            )?
        };
        let mut fibers = self.fibers.lock().unwrap();
        let id = FiberId(fibers.len());
        fibers.push(Arc::new(Fiber::new(id, handle)));
        drop(fibers);
        self.push_free_fiber(id);
        Ok(id)
    }
    pub(crate) fn fiber(&self, id: FiberId) -> Arc<Fiber> {
        self.fibers.lock().unwrap()[id.0].clone()
    }
    pub(crate) fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Acquire)
    }
    pub(crate) fn push_ready_fiber(&self, id: FiberId) {
        self.scheduler.lock().unwrap().ready.push_back(id);
        self.wake.notify_all();
    }
    pub(crate) fn push_free_fiber(&self, id: FiberId) {
        self.scheduler.lock().unwrap().free.push_back(id);
        self.wake.notify_all();
    }
    pub(crate) fn request_wakeups(&self, ids: Vec<FiberId>) {
        self.scheduler.lock().unwrap().wakes.extend(ids);
        self.wake.notify_all();
    }
    pub(crate) fn process_wake_requests(&self) {
        let ids: Vec<_> = self.scheduler.lock().unwrap().wakes.drain(..).collect();
        for id in ids {
            if self.fiber(id).request_wakeup() {
                self.push_ready_fiber(id);
            }
        }
    }
    pub(crate) fn next_ready_fiber(&self) -> Option<FiberId> {
        let (job, id) = {
            let mut state = self.scheduler.lock().unwrap();
            if let Some(id) = state.ready.pop_front() {
                return Some(id);
            }
            (state.jobs.pop_front()?, state.free.pop_front())
        };
        let id = match id {
            Some(id) => id,
            None => {
                // Grow stacks so waiting parents cannot exhaust the execution pool.
                if let Err(error) = self.allocate_fiber() {
                    self.complete(job.cancel(format!("fiber allocation failed: {error}")));
                    return None;
                }
                // Another worker may take the new fiber; requeue and retry.
                self.scheduler.lock().unwrap().jobs.push_front(job);
                self.wake.notify_all();
                return None;
            }
        };
        self.fiber(id).assign_job(job);
        Some(id)
    }
    pub(crate) fn complete(&self, completed: CompletedJob) {
        let CompletedJob {
            group,
            scope_group,
            result,
        } = completed;
        let failed = result.is_err();
        self.request_wakeups(group.finish(result));
        if let Some(scope) = scope_group {
            let result = if failed {
                Err(Box::new("scoped job failed") as Box<dyn std::any::Any + Send>)
            } else {
                Ok(())
            };
            self.request_wakeups(scope.finish(result));
        }
        self.scheduler.lock().unwrap().outstanding -= 1;
        self.wake.notify_all();
    }
    pub(crate) fn wait_for_work(&self) {
        let mut state = self.scheduler.lock().unwrap();
        while !self.is_shutdown()
            && state.jobs.is_empty()
            && state.ready.is_empty()
            && state.wakes.is_empty()
        {
            state = self.wake.wait(state).unwrap();
        }
    }
}
#[cfg(test)]
mod tests;
