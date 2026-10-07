use anyhow::Result;
use kani_volcano_math::Transform;
use kani_volcano_engine::{
    EntityApi, JobApi, Rotator, TimeApi,
    UpdateContext, UpdateSystem,
};

pub struct ParallelRotatorSystem;

impl UpdateSystem for ParallelRotatorSystem {
    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        let jobs = context.job_system()?;
        let dt = context.delta_seconds();

        let mut entries: Vec<_> = context
            .query2_mut::<Transform, Rotator>()
            .collect();

        unsafe {
            jobs.scope(|scope| -> Result<()> {
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
        Ok(())
    }
}