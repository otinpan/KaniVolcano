use std::sync::{
    Arc,
};
use super::fiber::{FiberHandle, FiberId};
use super::{JobSystemShared, Job};
use super::group::{JobGroup};


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WorkerId(pub usize);

pub struct Executor{
    worker_id: WorkerId, // worker thread id
    shared: Arc<JobSystemShared>, // thraed shared JobSystem

    manager_fiber: FiberHandle, 
    current_fiber: Option<FiberId>,
}

impl Executor{
    pub fn new(
        worker_id: WorkerId,
        shared: Arc<JobSystemShared>,
        manager_fiber: FiberHandle,
    ) -> Self{
        Self { worker_id, shared, manager_fiber, current_fiber: None }
    }

    pub fn run(&mut self) {
        loop {
            if self.shared.is_shutdown() {
                break;
            }

            if self.try_resume_ready_fiber() {
                continue;
            }

            if self.try_start_ready_job() {
                continue;
            }

            self.idle();
        }
    }

    pub fn suspend_current_fiber(
        &mut self,
        group: &JobGroup
    ){
        let fiber_id=self.current_fiber.expect("no current fiber");

        if !group.add_waiter(fiber_id){
            return;
        }

        unsafe{
            self.manager_fiber.switch_to();
        }
    }

    fn try_resume_ready_fiber(&mut self) -> bool {
        let Some(fiber_id) = self.shared.pop_ready_fiber(self.worker_id) else {
            return false;
        };

        self.resume_fiber(fiber_id);
        true
    }


    fn try_start_ready_job(&mut self) -> bool {
        let Some(job) = self.shared.pop_ready_job() else {
            return false;
        };

        let Some(fiber_id) = self.shared.pop_free_fiber(self.worker_id) else {
            self.shared.push_ready_job(job);
            return false;
        };

        self.assign_job(fiber_id, job);
        self.resume_fiber(fiber_id);

        true
    }

    fn resume_fiber(&mut self, fiber_id: FiberId) {
        self.current_fiber = Some(fiber_id);

        let handle = self.shared.fiber_handle(fiber_id);

        // do somethig
        unsafe {
            handle.switch_to();
        }

        self.current_fiber = None;
    }

    fn assign_job(
        &mut self,
        fiber_id: FiberId,
        job: Job,
    ) {
        self.shared.assign_job(fiber_id, job);
    }

    pub fn current_fiber(&self) -> Option<FiberId> {
        self.current_fiber
    }

    pub fn worker_id(&self) -> WorkerId {
        self.worker_id
    }

    pub fn manager_fiber(&self) -> &FiberHandle {
        &self.manager_fiber
    }

    fn idle(&self) {
        std::thread::yield_now();
    }

}