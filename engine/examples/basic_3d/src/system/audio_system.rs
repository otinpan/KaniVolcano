use std::collections::HashMap;

use anyhow::Result;
use cgmath::{InnerSpace, Vector3};
use kani_volcano_engine::prelude::*;
use kani_volcano_math::Transform;

use crate::component::SphereColliderComponent;

#[derive(Clone, Debug, Default)]
pub struct AudioSystem {
    // Emitter name -> (direct bus name, material bus names).
    pub buses: HashMap<String, (String, Vec<String>)>,
}

impl UpdateSystem for AudioSystem {
    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        let Some((_, transform, camera)) = context.query2::<Transform, Camera>().next() else {
            return Ok(());
        };
        let camera_pos = transform.position;
        let right = (camera.target - camera_pos).cross(camera.up);
        let camera_right = if right.magnitude2() > 0.0001 {
            right.normalize()
        } else {
            Vector3::new(0.0, 0.0, 0.0)
        };

        // Resolve names again each frame while audio registration is pending.
        let routes: HashMap<_, _> = self
            .buses
            .iter()
            .filter_map(|(name, buses)| context.audio_emitter_id(name).ok().map(|id| (id, buses)))
            .collect();
        let mut changes: Vec<(AudioBusId, f32, f32)> = Vec::new();

        for (_, transform, source) in context.query2::<Transform, AudioSource>() {
            let Some(emitter) = source.emitter else {
                continue;
            };
            let Some(&(direct_bus, material_buses)) = routes.get(&emitter) else {
                continue;
            };
            let audio_pos = transform.position;
            let offset = audio_pos - camera_pos;
            let distance_sq = offset.magnitude2();
            // Full volume within 10 world units, inverse-square attenuation beyond.
            let volume = (100.0 / distance_sq.max(0.0001)).min(1.0);
            let pan = if distance_sq > 0.0001 {
                offset.normalize().dot(camera_right).clamp(-1.0, 1.0)
            } else {
                0.0
            };

            let mut is_intersects=false;
            for bus_name in material_buses {
                let Some((tag, _)) = bus_name.rsplit_once('_') else {
                    continue;
                };
                let intersects = context.get_entities_by_tag(tag).into_iter().any(|entity| {
                    let Some(blocker) = context.get_component::<Transform>(entity) else {
                        return false;
                    };
                    let Some(collider) = context.get_component::<SphereColliderComponent>(entity)
                    else {
                        return false;
                    };
                    segment_intersects_sphere(
                        audio_pos,
                        camera_pos,
                        blocker.position,
                        collider.radius,
                    )
                    .0
                });
                if intersects{is_intersects=true};
                // Reset to zero when the segment no longer crosses this material.
                if let Ok(bus) = context.audio_bus_id(bus_name) {
                    changes.push((bus, if intersects { volume } else { 0.0 }, pan));
                }
            }
            if let Ok(bus) = context.audio_bus_id(direct_bus) {
                changes.push((bus, if is_intersects{volume*0.2} else{volume}, pan));
            }
        }

        for (bus, volume, pan) in changes {
            context.set_bus_volume_from_id(bus, volume, 0.0);
            context.set_bus_panning_from_id(bus, pan, 0.0);
        }
        Ok(())
    }
}

// Returns intersection status and the closest point on the segment.
// Radius is in world units, independent of Transform::scale.
fn segment_intersects_sphere(
    from: Vector3<f32>,
    to: Vector3<f32>,
    center: Vector3<f32>,
    radius: f32,
) -> (bool, Vector3<f32>) {
    let direction = to - from;
    let length_sq = direction.magnitude2();
    if length_sq == 0.0 {
        return ((center - from).magnitude2() <= radius * radius, from);
    }
    let t = ((center - from).dot(direction) / length_sq).clamp(0.0, 1.0);
    let closest = from + direction * t;
    ((center - closest).magnitude2() <= radius * radius, closest)
}
