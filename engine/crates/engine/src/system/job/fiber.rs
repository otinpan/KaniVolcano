use super::Job;
use super::executor::Executor;
use super::job::CompletedJob;
use std::sync::Mutex;
use std::{ffi::c_void, ptr::NonNull};

use anyhow::{Result, anyhow};

use windows_sys::Win32::System::Threading::{
    ConvertFiberToThread, ConvertThreadToFiberEx, CreateFiberEx, DeleteFiber,
};

// Windows SDK value. windows-sys exposes it under WindowsProgramming, whose
// feature is not enabled by the engine; keep dependencies unchanged here.
const FIBER_FLAG_FLOAT_SWITCH: u32 = 1;

pub(crate) type FiberEntry = unsafe extern "system" fn(*mut c_void);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FiberId(pub usize);

pub(crate) struct Fiber {
    id: FiberId,
    handle: FiberHandle,
    data: Mutex<FiberData>,
}

// Running -> Parking -> Waiting -> Ready -> Running
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FiberState {
    Free,
    Assigned,
    Running,
    Parking,
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
    wake_pending: bool, // Notification received before the manager finishes parking.
    completed: Option<CompletedJob>,
}

impl Fiber {
    pub(crate) fn new(id: FiberId, handle: FiberHandle) -> Self {
        assert!(
            handle.is_created_fiber(),
            "only created job fibers may be shared"
        );
        Self {
            id,
            handle,
            data: Mutex::new(FiberData {
                state: FiberState::Free,
                job: None,
                wake_pending: false,
                completed: None,
            }),
        }
    }

    pub(crate) fn handle(&self) -> &FiberHandle {
        &self.handle
    }

    pub(crate) fn id(&self) -> FiberId {
        self.id
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

    pub(crate) fn request_wakeup(&self) -> bool {
        let mut data = self.data.lock().unwrap();

        match data.state {
            FiberState::Parking => {
                data.wake_pending = true;
                false
            }
            FiberState::Waiting => {
                data.state = FiberState::Ready;
                true
            }
            _ => panic!("unexpected wakeup state: {:?}", data.state),
        }
    }

    pub(crate) fn prepare_resume(&self) {
        let mut data = self.data.lock().unwrap();

        match data.state {
            FiberState::Assigned => {
                // The first switch reaches take_job(), which changes to Running.
            }
            FiberState::Ready => {
                data.state = FiberState::Running;
            }
            _ => panic!("cannot resume fiber in {:?}", data.state),
        }
    }

    pub(crate) fn begin_parking(&self) {
        let mut data = self.data.lock().unwrap();

        assert_eq!(data.state, FiberState::Running);
        assert!(!data.wake_pending);

        data.state = FiberState::Parking;
    }

    // called when fiber returned to manager fiber
    // None: fiber is not pending
    // Some(true): fiber is pending and will be pushed into ready_fiber
    // Some(false): fiber is pending and waiting.
    pub(crate) fn finish_parking(&self) -> Option<bool> {
        let mut data = self.data.lock().unwrap();

        if data.state != FiberState::Parking {
            return None;
        }

        let ready = std::mem::take(&mut data.wake_pending);
        data.state = if ready {
            FiberState::Ready
        } else {
            FiberState::Waiting
        };

        Some(ready)
    }
    pub(crate) fn cancel_parking(&self) {
        let mut data = self.data.lock().unwrap();

        assert_eq!(data.state, FiberState::Parking);
        assert!(!data.wake_pending);

        data.state = FiberState::Running;
    }
}

// Pass this entry to CreateFiberEx; its parameter is unused. The manager must
// set current_fiber before the first switch and assign a job before each reuse.
// Job panics are captured by execute(); internal invariant failures abort at
// this non-unwinding Windows entry boundary.
pub(crate) unsafe extern "system" fn job_fiber_entry(_parameter: *mut c_void) {
    let id = Executor::current_job_fiber().expect("job fiber has no FiberId");
    loop {
        let shared = Executor::current_shared();
        // Assigned -> Running
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
pub(crate) struct FiberHandle {
    raw: NonNull<c_void>,
    kind: FiberKind,
}

impl FiberHandle {
    // create manager fiber.
    // this fiber is created in each threads.
    pub(crate) unsafe fn manager_fiber_from_thread() -> Result<Self> {
        let raw = unsafe { ConvertThreadToFiberEx(std::ptr::null_mut(), FIBER_FLAG_FLOAT_SWITCH) };

        let raw = NonNull::new(raw).ok_or_else(|| {
            anyhow!(
                "ConvertThreadToFiberEx failed: {}",
                std::io::Error::last_os_error()
            )
        })?;

        Ok(Self {
            raw,
            kind: FiberKind::ThreadFiber,
        })
    }

    // Reserve stack_size bytes (zero uses the executable default).
    pub(crate) unsafe fn create(
        stack_size: usize,
        entry: FiberEntry,
        parameter: *mut c_void,
    ) -> Result<Self> {
        let raw = unsafe {
            CreateFiberEx(
                0,
                stack_size,
                FIBER_FLAG_FLOAT_SWITCH,
                Some(entry),
                parameter,
            )
        };

        let raw = NonNull::new(raw)
            .ok_or_else(|| anyhow!("CreateFiberEx failed: {}", std::io::Error::last_os_error()))?;

        Ok(Self {
            raw,
            kind: FiberKind::Created,
        })
    }

    pub(crate) fn as_ptr(&self) -> *mut c_void {
        self.raw.as_ptr()
    }

    pub(crate) fn is_created_fiber(&self) -> bool {
        self.kind == FiberKind::Created
    }

    pub(crate) unsafe fn convert_back_to_thread(self) -> Result<()> {
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

// Only created job fibers enter this wrapper. The queues transfer exclusive
// execution ownership, and all stack migration occurs through unsafe wait/scope.
// The raw handle itself remains !Send/!Sync; manager handles never enter Fiber.
unsafe impl Send for Fiber {}
unsafe impl Sync for Fiber {}

impl Drop for Fiber {
    fn drop(&mut self) {
        assert_eq!(
            self.data.get_mut().unwrap().state,
            FiberState::Free,
            "cannot delete an active or suspended fiber"
        );
    }
}
impl Drop for FiberHandle {
    fn drop(&mut self) {
        // Converted manager fibers are explicitly converted back on their worker.
        // Created stacks are reclaimed only once all workers have stopped using them.
        if self.kind == FiberKind::Created {
            unsafe {
                DeleteFiber(self.raw.as_ptr());
            }
        }
    }
}
