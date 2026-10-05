use super::Job;
use super::executor::Executor;
use super::job::CompletedJob;
use std::sync::Mutex;
use std::{ffi::c_void, ptr::NonNull};

use anyhow::{Result, anyhow};

use windows_sys::Win32::System::Threading::{
    ConvertFiberToThread, ConvertThreadToFiber, CreateFiber, DeleteFiber, SwitchToFiber,
};

pub type FiberEntry = unsafe extern "system" fn(*mut c_void);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FiberId(pub usize);

pub struct Fiber {
    id: FiberId,
    handle: FiberHandle,
    data: Mutex<FiberData>,
}

impl Fiber {
    pub(crate) fn new(id: FiberId, handle: FiberHandle) -> Self {
        Self {
            id,
            handle,
            data: Mutex::new(FiberData {
                state: FiberState::Free,
                job: None,
                completed: None,
            }),
        }
    }

    pub(crate) fn handle(&self) -> &FiberHandle {
        &self.handle
    }

    pub(crate) fn assign_job(&self, job: Job) {
        let mut data = self.data.lock().unwrap();

        assert_eq!(data.state, FiberState::Free);
        assert!(data.job.is_none());
        assert!(data.completed.is_none());

        data.job = Some(job);
        data.state = FiberState::Assigned;
    }

    pub(crate) fn take_job(&self) -> Job {
        let mut data = self.data.lock().unwrap();

        assert_eq!(data.state, FiberState::Assigned);

        let job = data.job.take().expect("assigned fiber has no job");
        data.state = FiberState::Running;
        job
    }

    pub(crate) fn mark_finished(&self, completed: CompletedJob) {
        let mut data = self.data.lock().unwrap();

        assert_eq!(data.state, FiberState::Running);
        assert!(data.completed.is_none());
        data.completed = Some(completed);
        data.state = FiberState::Finished;
    }

    // Only the manager that just regained control may consume this result.
    pub(crate) fn take_completed(&self) -> Option<CompletedJob> {
        let mut data = self.data.lock().unwrap();
        if data.state != FiberState::Finished {
            return None;
        }
        data.completed.take()
    }

    pub(crate) fn recycle(&self) {
        let mut data = self.data.lock().unwrap();

        assert_eq!(data.state, FiberState::Finished);
        assert!(data.job.is_none());
        assert!(
            data.completed.is_none(),
            "consume completion before recycling"
        );

        data.state = FiberState::Free;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FiberState {
    Free,
    Assigned,
    Running,
    Waiting,
    Ready,
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FiberKind {
    Created,
    ThreadFiber,
}

struct FiberData {
    state: FiberState,
    job: Option<Job>,
    completed: Option<CompletedJob>,
}

// Pass this entry to CreateFiber; its parameter is unused. The manager must
// set current_fiber before the first switch and assign a job before each reuse.
// Job panics are captured by execute(); internal invariant failures abort at
// this non-unwinding Windows entry boundary.
pub(crate) unsafe extern "system" fn job_fiber_entry(_parameter: *mut c_void) {
    let id = Executor::current_job_fiber().expect("job fiber has no FiberId");
    loop {
        let shared = Executor::current_shared();
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

#[derive(Debug)]
pub struct FiberHandle {
    raw: NonNull<c_void>,
    kind: FiberKind,
}

impl FiberHandle {
    // create manager fiber.
    // this fiber is created in each threads.
    pub unsafe fn manager_fiber_from_thread() -> Result<Self> {
        let raw = unsafe { ConvertThreadToFiber(std::ptr::null_mut()) };

        let raw = NonNull::new(raw).ok_or_else(|| anyhow!("ConvertThreadToFiber failed"))?;

        Ok(Self {
            raw,
            kind: FiberKind::ThreadFiber,
        })
    }

    // crate new fiber for a job
    pub unsafe fn create(
        stack_size: usize,
        entry: FiberEntry,
        parameter: *mut c_void,
    ) -> Result<Self> {
        let raw = unsafe { CreateFiber(stack_size, Some(entry), parameter) };

        let raw = NonNull::new(raw).ok_or_else(|| anyhow!("CreateFiber failed"))?;

        Ok(Self {
            raw,
            kind: FiberKind::Created,
        })
    }

    // switch to this fiber
    // caller thread must be fiber.
    pub unsafe fn switch_to(&self) {
        unsafe {
            SwitchToFiber(self.raw.as_ptr());
        }
    }

    pub fn as_ptr(&self) -> *mut c_void {
        self.raw.as_ptr()
    }

    pub fn is_thread_fiber(&self) -> bool {
        self.kind == FiberKind::ThreadFiber
    }

    pub fn is_created_fiber(&self) -> bool {
        self.kind == FiberKind::Created
    }

    pub unsafe fn delete(self) {
        assert!(
            self.kind == FiberKind::Created,
            "thread fiber cannot be deleted with DeleteFiber"
        );

        unsafe {
            DeleteFiber(self.raw.as_ptr());
        }

        std::mem::forget(self);
    }

    pub unsafe fn convert_back_to_thread(self) -> Result<()> {
        assert!(
            self.kind == FiberKind::ThreadFiber,
            "only thread fiber can be converted back"
        );

        let ok = unsafe { ConvertFiberToThread() };

        if ok == 0 {
            return Err(anyhow!("ConvertFiberToThread failed"));
        }

        std::mem::forget(self);

        Ok(())
    }
}
