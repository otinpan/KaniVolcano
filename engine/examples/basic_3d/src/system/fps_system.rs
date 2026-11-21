use anyhow::Result;
use cgmath::{vec2, vec3};
use kani_volcano_engine::prelude::*;
use std::time::Instant;

/// Averages frame updates over half a second using unscaled wall-clock time.
pub struct FpsSystem {
    label: EntityId,
    since: Instant,
    frames: u32,
}

impl FpsSystem {
    pub fn install(context: &mut SceneContext<'_>) -> Result<()> {
        let label = context.spawn_text_ui2d(
            "eng_font", "FPS: -- | -- ms", 20.0, 24.0,
            vec2(-0.97, 0.85), vec2(1.0, 1.0), 0.0,
            vec3(1.0, 1.0, 0.3), 1.0,
        )?;
        context.add_update_system("fps", Self { label, since: Instant::now(), frames: 0 });
        Ok(())
    }
}

impl UpdateSystem for FpsSystem {
    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        let now = Instant::now();
        // Start timing at the first update, excluding scene initialization.
        if self.frames == 0 {
            self.since = now;
            self.frames = 1;
            return Ok(());
        }
        let seconds = now.duration_since(self.since).as_secs_f64();
        if seconds >= 0.5 {
            let fps = f64::from(self.frames) / seconds;
            if let Some(text) = context.get_component_mut::<Text>(self.label) {
                text.content = format!("FPS: {fps:.1} | {:.2} ms", 1000.0 / fps);
            }
            self.since = now;
            self.frames = 1;
        } else {
            self.frames += 1;
        }
        Ok(())
    }
}
