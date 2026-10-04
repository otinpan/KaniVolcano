mod group;
mod fiber;
mod executor;

use std::sync::{
    Arc, Mutex, Condvar,
    atomic::{AtomicBool},
};
use std::thread::{JoinHandle};
use std::collections::{VecDeque};
pub use fiber::{FiberId, FiberHandle};
pub use group::{JobGroup};


pub type JobTask=Box<dyn FnOnce() + Send + 'static>;

pub struct Job{
    task: JobTask,
    group: JobGroup,
}

pub struct JobSystem{
    shared: Arc<JobSystemShared>,
    workers: Vec<Worker>,
}

pub struct Worker{
    thread: JoinHandle<()>,
}

pub struct JobSystemShared{
    ready_jobs: Mutex<VecDeque<Job>>,
    ready_fibers: Mutex<VecDeque<FiberId>>,
    free_fibers: Mutex<VecDeque<FiberId>>,

    wake: Condvar,

    shutdown: AtomicBool,
}


impl JobSystem{
    pub fn new() -> Self{
        Self { 
            shared: JobSystemShared::new(), 
            workers: 
        }
    }

    pub fn pool
}

impl JobSystemShared{

}