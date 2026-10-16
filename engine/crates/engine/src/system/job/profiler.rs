use super::executor::WorkerId;
use super::fiber::FiberId;
use super::job::JobId;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Immutable identity shared by a job and its retained execution segments.
#[derive(Debug)]
pub struct JobInfo {
    pub id: JobId,
    pub name: Arc<str>,
}

/// One uninterrupted dispatch on a worker, excluding suspended wait time.
/// Times share the profiler's origin. This is elapsed time, not OS CPU time.
#[derive(Clone, Debug)]
pub struct JobRunSegment {
    pub job: Arc<JobInfo>,
    pub worker_id: WorkerId,
    pub fiber_id: FiberId,
    pub started_at: Duration,
    pub ended_at: Duration,
}

/// A bounded worker history. Reading it returns a copy, never a live lock.
#[derive(Clone, Debug)]
pub struct WorkerHistory {
    segments: VecDeque<JobRunSegment>,
    capacity: usize,
    dropped_segments: u64,
}

impl WorkerHistory {
    fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "history capacity must be positive");
        Self {
            segments: VecDeque::new(),
            capacity,
            dropped_segments: 0,
        }
    }

    fn push(&mut self, segment: JobRunSegment) {
        if self.segments.len() == self.capacity {
            self.segments.pop_front();
            self.dropped_segments = self.dropped_segments.saturating_add(1);
        }
        self.segments.push_back(segment);
    }

    /// Oldest to newest, including only dispatches that have returned to manager.
    pub fn segments(&self) -> &VecDeque<JobRunSegment> {
        &self.segments
    }
    /// Consumes a copied history without cloning its segments again.
    pub fn into_segments(self) -> VecDeque<JobRunSegment> {
        self.segments
    }
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    pub fn dropped_segments(&self) -> u64 {
        self.dropped_segments
    }
}

pub struct JobProfiler {
    origin: Instant,
    workers: Vec<Mutex<WorkerHistory>>,
}

impl JobProfiler {
    pub(crate) fn new(worker_count: usize, capacity_per_worker: usize) -> Self {
        Self {
            origin: Instant::now(),
            workers: (0..worker_count)
                .map(|_| Mutex::new(WorkerHistory::new(capacity_per_worker)))
                .collect(),
        }
    }

    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }
    pub fn elapsed(&self) -> Duration {
        self.origin.elapsed()
    }

    /// Copies one worker's bounded history. Returns None for an invalid index.
    /// Keeping many returned histories also keeps their segments/names alive.
    pub fn worker_history(&self, worker_index: usize) -> Option<WorkerHistory> {
        self.workers
            .get(worker_index)
            .map(|history| history.lock().unwrap().clone())
    }

    pub(crate) fn record(&self, segment: JobRunSegment) {
        self.workers[segment.worker_id.0]
            .lock()
            .unwrap()
            .push(segment);
    }

    /// Copies matching retained segments, sorted by start time. Each worker is
    /// read separately; this is not an atomic snapshot of all workers.
    pub fn get_job_segments_from_name(&self, name: &str) -> Vec<JobRunSegment> {
        let mut result = Vec::new();
        for worker in &self.workers {
            let history = worker.lock().unwrap();
            result.extend(
                history
                    .segments
                    .iter()
                    .filter(|segment| segment.job.name.as_ref() == name)
                    .cloned(),
            );
        }
        result.sort_unstable_by_key(|segment| segment.started_at);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eviction_is_bounded_and_snapshots_keep_metadata_alive() {
        let profiler = JobProfiler::new(2, 1);
        let info = Arc::new(JobInfo {
            id: JobId(1),
            name: Arc::from("first"),
        });
        let weak = Arc::downgrade(&info);
        profiler.record(JobRunSegment {
            job: info,
            worker_id: WorkerId(0),
            fiber_id: FiberId(0),
            started_at: Duration::ZERO,
            ended_at: Duration::from_millis(1),
        });
        let snapshot = profiler.worker_history(0).unwrap();
        profiler.record(JobRunSegment {
            job: Arc::new(JobInfo {
                id: JobId(2),
                name: Arc::from("second"),
            }),
            worker_id: WorkerId(0),
            fiber_id: FiberId(0),
            started_at: Duration::from_millis(2),
            ended_at: Duration::from_millis(3),
        });
        let latest = profiler.worker_history(0).unwrap();
        assert_eq!(latest.capacity(), 1);
        assert_eq!(latest.segments().len(), 1);
        assert_eq!(latest.segments()[0].job.id, JobId(2));
        assert_eq!(latest.dropped_segments(), 1);
        assert!(profiler.worker_history(1).unwrap().segments().is_empty());
        assert!(profiler.worker_history(2).is_none());
        assert!(weak.upgrade().is_some());
        drop(snapshot);
        assert!(weak.upgrade().is_none());
    }
}
