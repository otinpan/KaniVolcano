use std::{
    ffi::c_void,
    ptr::NonNull,
};

use anyhow::{anyhow, Result};

use windows_sys::Win32::System::Threading::{
    ConvertFiberToThread,
    ConvertThreadToFiber,
    CreateFiber,
    DeleteFiber,
    SwitchToFiber,
};

pub type FiberEntry =
    unsafe extern "system" fn(*mut c_void);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FiberId(pub usize);


pub struct Fiber{
    id: FiberId,
    handle: FiberHandle,
    state: FiberState,
}

pub enum FiberState{
    Free,
    Running,
    Waiting,
    Ready,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FiberKind {
    Created,
    ThreadFiber,
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
        let raw = unsafe {
            ConvertThreadToFiber(std::ptr::null_mut())
        };

        let raw = NonNull::new(raw)
            .ok_or_else(|| anyhow!("ConvertThreadToFiber failed"))?;

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
        let raw = unsafe {
            CreateFiber(
                stack_size,
                Some(entry),
                parameter,
            )
        };

        let raw = NonNull::new(raw)
            .ok_or_else(|| anyhow!("CreateFiber failed"))?;

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

        let ok = unsafe {
            ConvertFiberToThread()
        };

        if ok == 0 {
            return Err(anyhow!("ConvertFiberToThread failed"));
        }

        std::mem::forget(self);

        Ok(())
    }
}