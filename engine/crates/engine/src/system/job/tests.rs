use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

fn await_completion(group: &JobGroup) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !group.is_finished() {
        assert!(Instant::now() < deadline, "jobs failed to make progress");
        std::thread::yield_now();
    }
}

#[test]
fn owned_jobs_execute_and_shutdown_rejects_submissions() {
    let mut system = JobSystem::new(2, 2, 0).unwrap();
    let jobs = system.handle();
    let group = JobGroup::new();
    let count = Arc::new(AtomicUsize::new(0));
    for _ in 0..100 {
        let count = count.clone();
        jobs.spawn(&group, move || {
            count.fetch_add(1, Ordering::Relaxed);
        })
        .unwrap();
    }
    await_completion(&group);
    unsafe {
        jobs.wait(&group).unwrap();
    }
    assert_eq!(count.load(Ordering::Relaxed), 100);
    assert!(jobs.spawn(&group, || {}).is_err());
    system.shutdown().unwrap();
    assert!(system.shared.fibers.lock().unwrap().is_empty());
    assert!(jobs.spawn(&JobGroup::new(), || {}).is_err());
}

#[test]
fn one_worker_and_one_initial_fiber_can_wait_for_children() {
    let system = JobSystem::new(1, 1, 0).unwrap();
    let jobs = system.handle();
    let parent = JobGroup::new();
    let child_jobs = jobs.clone();
    let count = Arc::new(AtomicUsize::new(0));
    let child_count = count.clone();
    jobs.spawn(&parent, move || {
        let children = JobGroup::new();
        for _ in 0..3 {
            let count = child_count.clone();
            child_jobs
                .spawn(&children, move || {
                    count.fetch_add(1, Ordering::Relaxed);
                })
                .unwrap();
        }
        unsafe {
            child_jobs.wait(&children).unwrap();
        }
        assert_eq!(child_count.load(Ordering::Relaxed), 3);
    })
    .unwrap();
    await_completion(&parent);
    unsafe {
        jobs.wait(&parent).unwrap();
    }
    assert!(system.shared.fibers.lock().unwrap().len() >= 2);
}

#[test]
fn repeated_waits_and_reuse_across_workers() {
    let system = JobSystem::new(4, 2, 0).unwrap();
    let jobs = system.handle();
    let parents = JobGroup::new();
    let count = Arc::new(AtomicUsize::new(0));
    for _ in 0..16 {
        let jobs_for_parent = jobs.clone();
        let count = count.clone();
        jobs.spawn(&parents, move || {
            for _ in 0..20 {
                let group = JobGroup::new();
                for _ in 0..3 {
                    let count = count.clone();
                    jobs_for_parent
                        .spawn(&group, move || {
                            count.fetch_add(1, Ordering::Relaxed);
                        })
                        .unwrap();
                }
                unsafe {
                    jobs_for_parent.wait(&group).unwrap();
                }
            }
        })
        .unwrap();
    }
    await_completion(&parents);
    unsafe {
        jobs.wait(&parents).unwrap();
    }
    assert_eq!(count.load(Ordering::Relaxed), 16 * 20 * 3);
}

#[test]
fn scoped_jobs_borrow_disjoint_chunks() {
    let system = JobSystem::new(2, 1, 0).unwrap();
    let jobs = system.handle();
    let mut values = vec![0usize; 64];
    unsafe {
        jobs.scope(|scope| {
            let group = scope.group();
            for chunk in values.chunks_mut(8) {
                scope
                    .spawn(&group, move || {
                        chunk.fill(7);
                    })
                    .unwrap();
            }
            // Scope exit also waits when the caller omits explicit wait.
        })
        .unwrap();
    }
    assert!(values.iter().all(|value| *value == 7));
}

#[test]
fn scope_body_panic_still_drains_borrowed_jobs() {
    let system = JobSystem::new(1, 1, 0).unwrap();
    let jobs = system.handle();
    let mut value = 0;
    let result = catch_unwind(AssertUnwindSafe(|| unsafe {
        jobs.scope(|scope| {
            scope
                .spawn(&scope.group(), || {
                    value = 42;
                })
                .unwrap();
            panic!("scope body failed");
        })
    }));
    assert!(result.is_err());
    assert_eq!(value, 42);
}

#[test]
fn child_panic_is_reported_by_wait_and_scope() {
    let system = JobSystem::new(2, 1, 0).unwrap();
    let jobs = system.handle();
    let group = JobGroup::new();
    jobs.spawn(&group, || panic!("child failed")).unwrap();
    await_completion(&group);
    assert!(unsafe { jobs.wait(&group) }.is_err());
    assert!(
        unsafe {
            jobs.scope(|scope| {
                scope
                    .spawn(&scope.group(), || panic!("scoped child failed"))
                    .unwrap();
            })
        }
        .is_err()
    );
}

#[test]
fn nested_scoped_submissions_are_drained() {
    let system = JobSystem::new(2, 1, 0).unwrap();
    let jobs = system.handle();
    let value = AtomicUsize::new(0);
    unsafe {
        jobs.scope(|scope| {
            let value = &value;
            scope
                .spawn(&scope.group(), move || {
                    scope
                        .spawn(&scope.group(), move || {
                            value.store(1, Ordering::Relaxed);
                        })
                        .unwrap();
                })
                .unwrap();
        })
        .unwrap();
    }
    assert_eq!(value.load(Ordering::Relaxed), 1);
}

#[test]
fn groups_reject_another_system_and_sealed_submissions() {
    let first = JobSystem::new(1, 1, 0).unwrap();
    let second = JobSystem::new(1, 1, 0).unwrap();
    let group = JobGroup::new();
    first.handle().spawn(&group, || {}).unwrap();
    assert!(second.handle().spawn(&group, || {}).is_err());
    assert!(unsafe { second.handle().wait(&group) }.is_err());
    unsafe {
        first.handle().wait(&group).unwrap();
    }
    assert!(first.handle().spawn(&group, || {}).is_err());
}

#[test]
fn parking_notification_is_deferred_until_manager_returns() {
    let handle =
        unsafe { FiberHandle::create(0, fiber::job_fiber_entry, std::ptr::null_mut()).unwrap() };
    let fiber = Fiber::new(FiberId(0), handle);
    fiber.assign_job(Job::new_owned(|| {}, JobGroup::new()));
    let job = fiber.take_job();
    fiber.begin_parking();
    assert!(!fiber.request_wakeup());
    assert_eq!(fiber.finish_parking(), Some(true));
    fiber.prepare_resume();
    fiber.mark_finished(job.execute());
    fiber.take_completed().unwrap();
    fiber.recycle();
}

#[test]
fn concurrent_completions_wake_waiter_once() {
    let group = JobGroup::new();
    for _ in 0..16 {
        group.add(1).unwrap();
    }
    assert!(group.add_waiter(FiberId(7)));
    let waiters = std::thread::scope(|scope| {
        let threads: Vec<_> = (0..16)
            .map(|index| {
                let group = group.clone();
                scope.spawn(move || {
                    group.finish(if index % 2 == 0 {
                        Err(Box::new(index) as Box<dyn std::any::Any + Send>)
                    } else {
                        Ok(())
                    })
                })
            })
            .collect();
        threads
            .into_iter()
            .flat_map(|thread| thread.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(waiters, vec![FiberId(7)]);
    assert!(group.check_result().is_err());
}

#[test]
fn scoped_borrows_work_inside_a_waiting_job_fiber() {
    let system = JobSystem::new(2, 1, 0).unwrap();
    let jobs = system.handle();
    let parent = JobGroup::new();
    let child_jobs = jobs.clone();
    jobs.spawn(&parent, move || {
        let mut values = vec![0; 32];
        unsafe {
            child_jobs
                .scope(|scope| {
                    let group = scope.group();
                    for chunk in values.chunks_mut(4) {
                        scope
                            .spawn(&group, move || {
                                chunk.fill(9);
                            })
                            .unwrap();
                    }
                    scope.wait(&group).unwrap();
                })
                .unwrap();
        }
        assert!(values.iter().all(|value| *value == 9));
    })
    .unwrap();
    await_completion(&parent);
    unsafe {
        jobs.wait(&parent).unwrap();
    }
}

#[test]
fn waiting_fiber_resumes_on_another_worker() {
    // A child occupies worker B until a blocker starts on worker A. The blocker
    // then waits for the resumed parent, forcing B to execute that parent.
    let system = JobSystem::new(2, 2, 0).unwrap();
    let jobs = system.handle();
    let parent = JobGroup::new();
    let blockers = JobGroup::new();
    let parent_jobs = jobs.clone();
    let blocker_group = blockers.clone();
    jobs.spawn(&parent, move || {
        let before = Executor::current_worker_id();
        let children = JobGroup::new();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let (resumed_tx, resumed_rx) = std::sync::mpsc::channel();
        parent_jobs
            .spawn(&children, move || {
                started_tx.send(()).unwrap();
                release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            })
            .unwrap();
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        parent_jobs
            .spawn(&blocker_group, move || {
                release_tx.send(()).unwrap();
                resumed_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            })
            .unwrap();
        unsafe {
            parent_jobs.wait(&children).unwrap();
        }
        let after = Executor::current_worker_id();
        // Always release the blocker, even if the assertion below fails.
        resumed_tx.send(()).unwrap();
        assert_ne!(before, after);
    })
    .unwrap();
    await_completion(&parent);
    unsafe {
        jobs.wait(&parent).unwrap();
        jobs.wait(&blockers).unwrap();
    }
}
