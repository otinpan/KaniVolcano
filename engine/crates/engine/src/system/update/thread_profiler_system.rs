use super::{UpdateContext, UpdateSystem};
use crate::{
    AssetApi, EntityApi, EntityId, FontAssetId, JobApi, JobRunSegment, Material, MeshAssetId,
    MeshRenderer, ObjectApi, SceneId, SceneOwned, Text, TextPipeline, Visibility,
};
use anyhow::{Result, ensure};
use cgmath::{vec2, vec3};
use kani_volcano_math::Transform;
use renderer_vulkan::{PipelineKey, VertexLayout};
use std::time::{Duration, Instant};

const LEFT: f32 = -0.55;
const RIGHT: f32 = 0.95;
const TOP: f32 = 0.8;
const BOTTOM: f32 = -0.65;
// Bound draw objects independently of the much larger retained job history.
const TOTAL_BINS: usize = 512;
const MAX_BINS_PER_WORKER: usize = 256;

/// Screen-space timeline of retained, completed worker dispatches.
/// Both assets must be centered white 1x1 rectangles with Ui2D vertex layout.
/// line_mesh_id is drawn as a thin rectangle, not a DebugLine3D line-list mesh.
///
/// Register with add_update_system, after the systems being observed. Labels use
/// the first loaded font, if any. Set scene_id for scene-owned overlay entities.
/// Bars are quantized to a bounded number of time bins; colors group job names.
/// This is a timeline of recorded job dispatches, not measured OS CPU utilization.
pub struct ThreadProfilerSystem {
    pub rect_mesh_id: MeshAssetId,
    pub line_mesh_id: MeshAssetId,
    pub refresh_interval: Duration,
    pub time_window: Duration,
    pub scene_id: Option<SceneId>,
    last_refresh: Option<Instant>,
    rectangles: Vec<EntityId>,
    lines: Vec<EntityId>,
    labels: Vec<EntityId>,
}

impl ThreadProfilerSystem {
    pub fn new(rect_mesh_id: MeshAssetId, line_mesh_id: MeshAssetId) -> Self {
        Self {
            rect_mesh_id,
            line_mesh_id,
            refresh_interval: Duration::from_millis(100),
            time_window: Duration::from_secs(1),
            scene_id: None,
            last_refresh: None,
            rectangles: Vec::new(),
            lines: Vec::new(),
            labels: Vec::new(),
        }
    }

    /// Remove this system's overlay before removing the system itself.
    /// Alternatively, assign scene_id and let scene cleanup remove the entities.
    pub fn clear(&mut self, context: &mut UpdateContext<'_>) {
        for id in self
            .rectangles
            .drain(..)
            .chain(self.lines.drain(..))
            .chain(self.labels.drain(..))
        {
            context.despawn(id);
        }
        self.last_refresh = None;
    }

    fn validate_mesh(context: &UpdateContext<'_>, id: MeshAssetId) -> Result<()> {
        let layout = context
            .mesh_assets()
            .find(|(asset_id, _)| *asset_id == id)
            .map(|(_, asset)| asset.handle.vertex_layout);
        ensure!(
            layout == Some(VertexLayout::Ui2D),
            "ThreadProfilerSystem mesh {:?} must be a centered white 1x1 Ui2D rectangle",
            id
        );
        Ok(())
    }

    fn tag(context: &mut UpdateContext<'_>, id: EntityId, scene_id: Option<SceneId>) {
        context.set_tags(id, ["ThreadProfiler"]);
        if let Some(scene_id) = scene_id {
            context.add_component(id, SceneOwned { scene_id });
        }
    }

    // Reuse the supplied mesh through ObjectApi; never create GPU meshes here.
    fn spawn_rectangle_from_mesh(
        context: &mut UpdateContext<'_>,
        mesh: MeshAssetId,
        scene_id: Option<SceneId>,
    ) -> Result<EntityId> {
        let id = context.spawn_primitive_from_mesh(
            mesh,
            Material {
                pipeline_key: PipelineKey::Ui2D,
                ..Material::default()
            },
            Transform::default(),
        )?;
        Self::tag(context, id, scene_id);
        Ok(id)
    }

    fn rectangle(
        context: &mut UpdateContext<'_>,
        mesh: MeshAssetId,
        pool: &mut Vec<EntityId>,
        used: &mut usize,
        scene_id: Option<SceneId>,
        center: cgmath::Vector2<f32>,
        size: cgmath::Vector2<f32>,
        color: cgmath::Vector3<f32>,
    ) -> Result<()> {
        let existing = pool.get(*used).copied().filter(|id| {
            context
                .get_component::<MeshRenderer>(*id)
                .is_some_and(|renderer| renderer.asset_id == Some(mesh))
                && context.has_component::<Transform>(*id)
                && context.has_component::<Visibility>(*id)
        });
        let id = if let Some(id) = existing {
            id
        } else {
            if let Some(&old) = pool.get(*used) {
                context.despawn(old);
            }
            let id = Self::spawn_rectangle_from_mesh(context, mesh, scene_id)?;
            if *used == pool.len() {
                pool.push(id);
            } else {
                pool[*used] = id;
            }
            id
        };
        // Ui2D uses position.yz and scale.yz; its vertex shader ignores rotation.
        *context.get_component_mut::<Transform>(id).unwrap() = Transform {
            position: vec3(0.0, center.x, center.y),
            scale: vec3(1.0, size.x, size.y),
            ..Transform::default()
        };
        context
            .get_component_mut::<MeshRenderer>(id)
            .unwrap()
            .material
            .color = color;
        context
            .get_component_mut::<Visibility>(id)
            .unwrap()
            .is_visible = true;
        *used += 1;
        Ok(())
    }

    fn label(
        &mut self,
        context: &mut UpdateContext<'_>,
        used: &mut usize,
        font: FontAssetId,
        content: String,
        x: f32,
        y: f32,
        font_size: f32,
    ) {
        let id = if let Some(id) = self.labels.get(*used).copied().filter(|id| {
            context.has_component::<Text>(*id)
                && context.has_component::<Transform>(*id)
                && context.has_component::<Visibility>(*id)
        }) {
            id
        } else {
            if let Some(&old) = self.labels.get(*used) {
                context.despawn(old);
            }
            let id = context.spawn();
            Self::tag(context, id, self.scene_id);
            context.add_component(id, Transform::default());
            context.add_component(id, Visibility::default());
            context.add_component(
                id,
                Text {
                    content: String::new(),
                    font,
                    font_size,
                    line_height: font_size,
                    color: vec3(0.9, 0.9, 0.9),
                    alpha: 1.0,
                    text_pipeline: TextPipeline::Ui2D,
                },
            );
            if *used == self.labels.len() {
                self.labels.push(id);
            } else {
                self.labels[*used] = id;
            }
            id
        };
        let text = context.get_component_mut::<Text>(id).unwrap();
        if text.content != content {
            text.content = content;
        }
        text.font = font;
        text.font_size = font_size;
        text.line_height = font_size;
        // TextUi2D anchors use XY, with positive Y downward (unlike Ui2D rectangles).
        context.get_component_mut::<Transform>(id).unwrap().position = vec3(x, -y, 0.0);
        context
            .get_component_mut::<Visibility>(id)
            .unwrap()
            .is_visible = true;
        *used += 1;
    }

    fn hide_unused(context: &mut UpdateContext<'_>, pool: &[EntityId], used: usize) {
        for &id in &pool[used..] {
            if let Some(visibility) = context.get_component_mut::<Visibility>(id) {
                visibility.is_visible = false;
            }
        }
    }
}

impl UpdateSystem for ThreadProfilerSystem {
    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        let now = Instant::now();
        if self
            .last_refresh
            .is_some_and(|last| now.duration_since(last) < self.refresh_interval)
        {
            return Ok(());
        }
        ensure!(
            !self.time_window.is_zero(),
            "profiler time_window must be positive"
        );
        Self::validate_mesh(context, self.rect_mesh_id)?;
        Self::validate_mesh(context, self.line_mesh_id)?;

        let jobs = context.job_system()?;
        let worker_count = context.worker_count()?;
        // Capture once so every worker uses the same time window.
        let end = jobs.profiler().elapsed();
        let font = context.font_assets().next().map(|(id, _)| id);
        let row_height = (TOP - BOTTOM) / worker_count.max(1) as f32;
        let bin_count = (TOTAL_BINS / worker_count.max(1)).clamp(1, MAX_BINS_PER_WORKER);
        let mut bins = vec![None; bin_count];
        let (mut rectangles, mut lines, mut labels) = (0, 0, 0);

        for worker in 0..worker_count {
            let history = context.worker_history(worker)?;
            fill_bins(history.iter(), end, self.time_window, &mut bins);
            let y = TOP - (worker as f32 + 0.5) * row_height;
            Self::rectangle(
                context,
                self.rect_mesh_id,
                &mut self.rectangles,
                &mut rectangles,
                self.scene_id,
                vec2((LEFT + RIGHT) * 0.5, y),
                vec2(RIGHT - LEFT, row_height * 0.75),
                vec3(0.06, 0.07, 0.09),
            )?;

            let mut bin = 0;
            while bin < bin_count {
                let Some(color_index) = bins[bin] else {
                    bin += 1;
                    continue;
                };
                let start = bin;
                while bin < bin_count && bins[bin] == Some(color_index) {
                    bin += 1;
                }
                let left = LEFT + (RIGHT - LEFT) * start as f32 / bin_count as f32;
                let right = LEFT + (RIGHT - LEFT) * bin as f32 / bin_count as f32;
                Self::rectangle(
                    context,
                    self.rect_mesh_id,
                    &mut self.rectangles,
                    &mut rectangles,
                    self.scene_id,
                    vec2((left + right) * 0.5, y),
                    vec2(right - left, row_height * 0.65),
                    job_color(color_index),
                )?;
            }
            if let Some(font) = font {
                self.label(
                    context,
                    &mut labels,
                    font,
                    format!("Worker {worker}"),
                    -0.98,
                    y + row_height * 0.15,
                    (row_height * 300.0).clamp(4.0, 12.0),
                );
            }
        }

        // Six grid lines, with labels relative to the captured window end.
        for tick in 0..=5 {
            let fraction = tick as f32 / 5.0;
            let x = LEFT + (RIGHT - LEFT) * fraction;
            Self::rectangle(
                context,
                self.line_mesh_id,
                &mut self.lines,
                &mut lines,
                self.scene_id,
                vec2(x, (TOP + BOTTOM) * 0.5),
                vec2(0.001, TOP - BOTTOM + 0.02),
                vec3(0.25, 0.27, 0.30),
            )?;
            if let Some(font) = font {
                let ms = self.time_window.as_secs_f64() * 1000.0 * (1.0 - f64::from(fraction));
                self.label(
                    context,
                    &mut labels,
                    font,
                    format!("-{ms:.0} ms"),
                    x - 0.04,
                    BOTTOM - 0.04,
                    11.0,
                );
            }
        }
        Self::rectangle(
            context,
            self.line_mesh_id,
            &mut self.lines,
            &mut lines,
            self.scene_id,
            vec2((LEFT + RIGHT) * 0.5, BOTTOM - 0.01),
            vec2(RIGHT - LEFT, 0.002),
            vec3(0.5, 0.5, 0.5),
        )?;
        Self::hide_unused(context, &self.rectangles, rectangles);
        Self::hide_unused(context, &self.lines, lines);
        Self::hide_unused(context, &self.labels, labels);
        self.last_refresh = Some(Instant::now());
        Ok(())
    }
}

// Quantize retained intervals to display bins so thousands of jobs do not become
// thousands of Vulkan draw objects. A filled bin means some work occurred there,
// not that the worker was occupied for the entire bin. Later intervals win ties.
fn fill_bins<'a>(
    segments: impl Iterator<Item = &'a JobRunSegment>,
    end: Duration,
    window: Duration,
    bins: &mut [Option<usize>],
) {
    bins.fill(None);
    let seconds = window.as_secs_f64();
    let window_start = end.as_secs_f64() - seconds;
    let bin_count = bins.len();
    for segment in segments {
        let left = ((segment.started_at.as_secs_f64() - window_start) / seconds).clamp(0.0, 1.0);
        let right = ((segment.ended_at.as_secs_f64() - window_start) / seconds).clamp(0.0, 1.0);
        if right <= left {
            continue;
        }
        let first = (left * bin_count as f64).floor() as usize;
        let last = ((right * bin_count as f64).ceil() as usize).min(bin_count);
        let hash = segment.job.name.bytes().fold(0u64, |hash, byte| {
            hash.wrapping_mul(31).wrapping_add(u64::from(byte))
        });
        bins[first..last].fill(Some((hash % 6) as usize));
    }
}

fn job_color(index: usize) -> cgmath::Vector3<f32> {
    [
        vec3(0.2, 0.7, 1.0),
        vec3(0.3, 0.9, 0.5),
        vec3(1.0, 0.7, 0.2),
        vec3(0.8, 0.4, 1.0),
        vec3(1.0, 0.4, 0.5),
        vec3(0.2, 0.9, 0.9),
    ][index]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::api::register_job_system;
    use crate::{FiberId, JobId, JobInfo, JobSystem, WorkerId};
    use std::sync::Arc;

    fn segment(start_ms: u64, end_ms: u64) -> JobRunSegment {
        JobRunSegment {
            job: Arc::new(JobInfo {
                id: JobId(1),
                name: Arc::from("rotate"),
            }),
            worker_id: WorkerId(0),
            fiber_id: FiberId(0),
            started_at: Duration::from_millis(start_ms),
            ended_at: Duration::from_millis(end_ms),
        }
    }

    #[test]
    fn timeline_clips_to_window_and_preserves_idle_gaps() {
        let segments = [segment(0, 750), segment(1250, 2000), segment(0, 100)];
        let mut bins = vec![None; 8];
        fill_bins(
            segments.iter(),
            Duration::from_millis(1500),
            Duration::from_secs(1),
            &mut bins,
        );
        assert!(bins[..2].iter().all(Option::is_some));
        assert!(bins[2..6].iter().all(Option::is_none));
        assert!(bins[6..].iter().all(Option::is_some));
        fill_bins(
            std::iter::empty(),
            Duration::from_secs(3),
            Duration::from_secs(1),
            &mut bins,
        );
        assert!(bins.iter().all(Option::is_none));
    }

    fn with_context(test: impl FnOnce(&mut UpdateContext<'_>, MeshAssetId, MeshAssetId)) {
        let jobs = JobSystem::new(2, 2, 0).unwrap();
        let _guard = register_job_system(jobs.handle());
        let mut world = crate::World::default();
        let input = crate::Input::default();
        let time = crate::Time::default();
        let mut resources = crate::Resources::default();
        let rect = resources.insert_mesh_asset(
            renderer_vulkan::MeshHandle::new(0, VertexLayout::Ui2D),
            false,
        );
        let line = resources.insert_mesh_asset(
            renderer_vulkan::MeshHandle::new(1, VertexLayout::Ui2D),
            false,
        );
        let mut render = crate::RenderCommandQueue::default();
        let mut assets = crate::AssetLoadCommandQueue::default();
        let mut audio = crate::AudioCommandQueue::default();
        let mut scene = crate::SceneCommandQueue::default();
        let mut context = UpdateContext::new(
            &mut world,
            &input,
            &time,
            &mut resources,
            &mut render,
            &mut assets,
            &mut audio,
            &mut scene,
        );
        test(&mut context, rect, line);
    }

    #[test]
    fn refresh_reuses_entities_and_hides_expired_segments() {
        with_context(|context, rect, line| {
            let jobs = context.job_system().unwrap();
            let mut dispatch = segment(0, 0);
            dispatch.ended_at = jobs.profiler().elapsed();
            jobs.profiler().record(dispatch);
            let mut system = ThreadProfilerSystem::new(rect, line);
            system.refresh_interval = Duration::ZERO;
            system.scene_id = Some(SceneId(42));
            system.update(context).unwrap();
            let entity_count = context.entity_count();
            assert_eq!(system.rectangles.len(), 3); // Two rows and one active bar.
            assert!(system.labels.is_empty()); // No font dependency.
            assert!(system.rectangles.iter().chain(&system.lines).all(|id| {
                context.get_component::<SceneOwned>(*id).unwrap().scene_id == SceneId(42)
            }));

            system.update(context).unwrap();
            assert_eq!(context.entity_count(), entity_count);
            // The old dispatch lies outside this new tiny window.
            system.time_window = Duration::from_nanos(1);
            system.update(context).unwrap();
            assert_eq!(context.entity_count(), entity_count);
            assert_eq!(
                system
                    .rectangles
                    .iter()
                    .filter(|id| context
                        .get_component::<Visibility>(**id)
                        .unwrap()
                        .is_visible)
                    .count(),
                2
            );

            // Refresh is gated by real time even though simulation Time stays zero.
            let last_refresh = system.last_refresh;
            system.refresh_interval = Duration::MAX;
            system.update(context).unwrap();
            assert_eq!(system.last_refresh, last_refresh);
            system.clear(context);
            assert_eq!(context.entity_count(), 0);
            assert!(context.mesh_assets().all(|(_, asset)| asset.ref_count == 0));
        });
    }

    #[test]
    fn invalid_window_and_mesh_fail_before_creating_entities() {
        with_context(|context, rect, line| {
            let mut system = ThreadProfilerSystem::new(rect, line);
            system.time_window = Duration::ZERO;
            assert!(system.update(context).is_err());
            system.time_window = Duration::from_secs(1);
            system.line_mesh_id = MeshAssetId(usize::MAX);
            assert!(system.update(context).is_err());
            assert_eq!(context.entity_count(), 0);
        });
    }
}
