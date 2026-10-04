use std::sync::{Arc, Mutex};

use super::FiberId;

#[derive(Clone, Default)]
pub struct JobGroup {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    pending: usize,
    waiters: Vec<FiberId>,
}

impl JobGroup {
    pub fn new() -> Self {
        Self::default()
    }

    // Adds one job to this group.
    // This must be called before the job is inserted into the ready queue.
    pub(crate) fn add(&self) {
        let mut state = self.inner.state.lock().unwrap();

        state.pending = state
            .pending
            .checked_add(1)
            .expect("job group pending count overflow");
    }

    // Marks one job in this group as completed.
    // Returns the fibers that should become ready when this completion
    // changes the pending count to zero.
    pub(crate) fn finish(&self) -> Vec<FiberId> {
        let mut state = self.inner.state.lock().unwrap();

        assert!(
            state.pending > 0,
            "JobGroup::finish() called with no pending jobs"
        );

        state.pending -= 1;

        if state.pending == 0 {
            std::mem::take(&mut state.waiters)
        } else {
            Vec::new()
        }
    }

    // Registers a fiber as waiting for this group.
    // Returns `true` if the fiber must suspend.
    // Returns `false` if all jobs have already completed.
    pub(crate) fn add_waiter(&self, fiber: FiberId) -> bool {
        let mut state = self.inner.state.lock().unwrap();

        if state.pending == 0 {
            return false;
        }

        state.waiters.push(fiber);
        true
    }

    pub fn is_finished(&self) -> bool {
        let state = self.inner.state.lock().unwrap();
        state.pending == 0
    }

    pub fn pending(&self) -> usize {
        let state = self.inner.state.lock().unwrap();
        state.pending
    }
}