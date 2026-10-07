use anyhow::Result;
use kani_volcano_math::Transform;
use std::time::{Duration, Instant};

use super::{UpdateContext, UpdateSystem};
use crate::Rotator;
use crate::{EntityApi, TimeApi};

#[derive(Clone, Debug, Default)]
pub struct RotatorSystem {
    total_update_time: Duration,
    update_count: u32,
}

impl UpdateSystem for RotatorSystem {
    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        let start = Instant::now();
        let delta_time = context.delta_seconds();

        for (_, transform, rotator) in context.query2_mut::<Transform, Rotator>() {
            transform.rotate(rotator.speed * delta_time);
        }

        self.total_update_time += start.elapsed();
        self.update_count += 1;
        if self.update_count == 120 {
            println!(
                "RotatorSystem: average {:.3} ms/update (120 updates)",
                self.total_update_time.as_secs_f64() * 1000.0 / f64::from(self.update_count),
            );
            self.total_update_time = Duration::ZERO;
            self.update_count = 0;
        }

        Ok(())
    }
}
