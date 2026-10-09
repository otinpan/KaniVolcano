use anyhow::Result;
use kani_volcano_engine::{EntityApi, JobApi, Rotator, TimeApi, UpdateContext, UpdateSystem};
use kani_volcano_math::Transform;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct ParallelRotatorSystem {
    total_update_time: Duration,
    update_count: u32,
}

impl UpdateSystem for ParallelRotatorSystem {
    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        let start = Instant::now();
        let jobs = context.job_system()?;
        let dt = context.delta_seconds();

        let mut entries: Vec<_> = context.query2_mut::<Transform, Rotator>().collect();

        unsafe {
        pe(|scope| -> Result<()> {
                let group = scope.group();

                for chunk in entries.chunks_mut(256) {
                    scope.spawn(&group, move || {
                        for (_, transform, rotator) in chunk {
                            transform.rotate(rotator.speed * dt);
                        }
                    })?;
                }

                scope.wait(&group)
            })
        }??;
        // Include releasing the temporary query buffer in the measured cost.
        drop(entries);
        self.total_update_time += start.elapsed();
        self.update_count += 1;
        if self.update_count == 120 {
            println!(
                "ParallelRotatorSystem: average {:.3} ms/update (120 updates)",
                self.total_update_time.as_secs_f64() * 1000.0 / f64::from(self.update_count),
            );
            self.total_update_time = Duration::ZERO;
            self.update_count = 0;
        }
        Ok(())
    }
}
