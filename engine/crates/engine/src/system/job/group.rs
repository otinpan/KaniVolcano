use super::FiberId;
use anyhow::{Result, ensure};
use std::any::Any;
use std::sync::{Arc, Condvar, Mutex};

#[derive(Clone, Default)]
pub struct JobGroup {
    inner: Arc<Inner>,
}
#[derive(Default)]
struct Inner {
    state: Mutex<State>,
    completed: Condvar,
}
#[derive(Default)]
struct State {
    pending: usize,
    waiters: Vec<FiberId>,
    failures: Vec<Box<dyn Any + Send + 'static>>,
    owner: Option<usize>,
    sealed: bool,
}
impl JobGroup {
    pub fn new() -> Self {
        Self::default()
    }
    fn bind(state: &mut State, owner: usize) -> Result<()> {
        ensure!(
            state.owner.is_none_or(|id| id == owner),
            "group belongs to another JobSystem"
        );
        state.owner = Some(owner);
        Ok(())
    }
    pub(crate) fn add(&self, owner: usize) -> Result<()> {
        let mut state = self.inner.state.lock().unwrap();
        Self::bind(&mut state, owner)?;
        ensure!(!state.sealed, "cannot submit to a group after wait");
        state.pending = state.pending.checked_add(1).expect("job count overflow");
        Ok(())
    }
    pub(crate) fn seal(&self, owner: usize) -> Result<()> {
        let mut state = self.inner.state.lock().unwrap();
        Self::bind(&mut state, owner)?;
        state.sealed = true;
        Ok(())
    }
    pub(crate) fn finish(&self, result: std::thread::Result<()>) -> Vec<FiberId> {
        let mut state = self.inner.state.lock().unwrap();
        assert!(state.pending > 0, "completion without pending job");
        if let Err(error) = result {
            state.failures.push(error);
        }
        state.pending -= 1;
        if state.pending == 0 {
            self.inner.completed.notify_all();
            std::mem::take(&mut state.waiters)
        } else {
            Vec::new()
        }
    }
    pub(crate) fn add_waiter(&self, id: FiberId) -> bool {
        let mut state = self.inner.state.lock().unwrap();
        if state.pending == 0 {
            return false;
        }
        state.waiters.push(id);
        true
    }
    pub(crate) fn wait_blocking(&self) {
        let mut state = self.inner.state.lock().unwrap();
        while state.pending != 0 {
            state = self.inner.completed.wait(state).unwrap();
        }
    }
    pub(crate) fn check_result(&self) -> Result<()> {
        let state = self.inner.state.lock().unwrap();
        assert_eq!(state.pending, 0, "unfinished jobs");
        ensure!(
            state.failures.is_empty(),
            "{} jobs failed",
            state.failures.len()
        );
        Ok(())
    }
    pub fn pending(&self) -> usize {
        self.inner.state.lock().unwrap().pending
    }
    pub fn is_finished(&self) -> bool {
        self.pending() == 0
    }
}
