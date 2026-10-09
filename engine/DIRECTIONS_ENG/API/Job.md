# JobApi

Parallelism means executing multiple operations at the same time, for example on multiple CPU cores. Concurrency means making progress on multiple operations. Concurrent work can be interleaved on one core or run in parallel on multiple cores.

For example, one core can perform collision detection while another computes AI decisions. Two cores can also share a large batch of collision tests. When tasks depend on each other, their execution order and synchronization must be managed.

Concurrency also allows other work to progress while a file download waits for I/O. Running other tasks during these waits can improve CPU utilization. However, blocking I/O inside a job is not automatically converted into a fiber wait.

## Workers and job fibers

By default, KaniVolcanoEngine creates one fewer worker thread than the available parallelism, with a minimum of one worker. The initial job fiber count is four times the worker count.

A worker is a thread that executes jobs, where a job is one unit of work. Each worker converts its own thread into a manager fiber during startup. This manager fiber belongs to that worker and is separate from the shared job fiber pool.

Submitted jobs enter a shared queue and are assigned to free job fibers. A worker switches from its manager fiber to a job fiber to execute the task. When the job completes or waits for other jobs, it returns to the manager fiber, allowing the worker to execute other work. A waiting job fiber may resume on another worker after its dependencies finish. Additional job fibers are created if the pool runs out of free fibers.

Fiber switching is managed by the application. Job creation, queue submission, synchronization, and fiber switching still have costs, so parallelizing every operation does not necessarily make it faster.

![Worker and job fiber execution flow](../assets/job_system_explanation.png)

## Example

The following `ParallelRotatorSystem` rotates objects with a `Rotator` component in parallel. The engine's `Rotator` stores rotation speed as follows:

```rust
use cgmath::Vector3;
use kani_volcano_engine::Component;

pub struct Rotator {
    pub speed: Vector3<f32>,
}

impl Component for Rotator {}
```

The system below imports and uses the engine-provided `Rotator`.

```rust
use anyhow::Result;
use kani_volcano_engine::{EntityApi, JobApi, Rotator, TimeApi, UpdateContext, UpdateSystem};
use kani_volcano_math::Transform;

#[derive(Default)]
pub struct ParallelRotatorSystem {}

impl UpdateSystem for ParallelRotatorSystem {
    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        // Get an owned JobSystemHandle before borrowing components.
        let jobs = context.job_system()?;
        let dt = context.delta_seconds();

        let mut entries: Vec<_> = context.query2_mut::<Transform, Rotator>().collect();

        unsafe {
            jobs.scope(|scope| -> Result<()> {
                // Processing before submitting child jobs runs on the calling thread.

                // Create a group to track these child jobs.
                let group = scope.group();

                for chunk in entries.chunks_mut(256) {
                    scope.spawn(&group, move || {
                        // This closure runs on a worker.
                        for (_, transform, rotator) in chunk {
                            transform.rotate(rotator.speed * dt);
                        }
                    })?;
                }

                // Wait for all jobs in this group to finish.
                scope.wait(&group)?;

                // Processing after the wait runs on the calling thread.

                Ok(())
            })
        }??;
        Ok(())
    }
}
```

`context.job_system()` returns a `JobSystemHandle` for the engine's job system. During update and fixed update stages, the Scheduler registers a handle in the calling thread's TLS. Obtaining the handle does not retain a borrow of the Context, so components can be borrowed mutably afterward.

1. `jobs.scope(...)` starts a scope for jobs that borrow data. All jobs submitted through that scope finish before it exits, even if an explicit `wait()` is omitted.
2. `scope.group()` creates a group that tracks completion of a set of jobs.
3. `scope.spawn(&group, func)` submits `func` (`FnOnce() + Send`) to the shared job queue and registers it with the group. This example gives each job a disjoint chunk of up to 256 entries.
4. `scope.wait(&group)` waits for every job in the group to finish. Waiting seals the group against further submissions; use a new group for subsequent jobs.

For example, to run jobs B, C, and D in parallel after processing A, then use their results in processing E, register B, C, and D in the same group. Run A, submit the child jobs, and run E after `scope.wait(&group)?`. The scope closure itself is not automatically submitted as a job.

An `update()` called by the current Scheduler executes on the main thread. In this example, the main thread waits for the child jobs, then continues with subsequent processing. When a job running on a worker calls `wait()`, its job fiber suspends and the worker can execute other jobs.

The final `??` propagates both errors from `scope()` itself and the `Result` returned by its closure.

## Unsafe requirements

Currently, `scope()` and `wait()` are `unsafe`. When called on a worker, the suspended fiber may resume on another worker. Every live value and reference on the entire suspended stack, including caller frames, must support this migration. Do not keep thread-affine guards, TLS references, or non-`Send` values across a wait. A `Send` closure does not guarantee that local variables created inside it can migrate.

Also avoid waiting on a group that includes the waiting job itself, and avoid cyclic dependencies.

## Work suited to parallel execution

This `ParallelRotatorSystem` demonstrates API usage; it is not a good candidate for speeding up lightweight rotation updates. Each object update mainly multiplies rotation speed by elapsed time and adds the result to the rotation angles. It does not perform a matrix calculation during the update.

When the computation per job is small, submission, synchronization, and fiber switching can cost more than the time saved by parallel execution. This example also collects query results into a `Vec` serially. In our measurements, the sequential `RotatorSystem` was faster.

Try increasing the number of entries per job to reduce the job count, or target more computationally expensive work such as:

- Animation interpolation and bone matrix calculations
- AI pathfinding
- Collision tests for independent candidate pairs
- Terrain or mesh vertex, index, and normal generation

Measure the results against a sequential implementation. Split mutable data into disjoint regions, and perform thread-affine operations such as GPU resource registration on the appropriate thread after gathering the computed results.
