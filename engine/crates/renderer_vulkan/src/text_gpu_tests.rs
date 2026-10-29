//! Opt-in real GPU check: cargo test -p renderer_vulkan text_gpu_tests -- --ignored --nocapture
//! Requires Vulkan validation layers and KANI_TEST_FONT (defaults to Windows Arial).
use crate::{GpuTextMesh, PipelineKey, TextRenderItem, VulkanRenderer};
use anyhow::Result;
use kani_volcano_math::Transform;
use std::sync::atomic::{AtomicUsize, Ordering};
use vulkanalia::vk::InstanceV1_0;
use winit::{
    event_loop::EventLoopBuilder, platform::windows::EventLoopBuilderExtWindows,
    window::WindowBuilder,
};

struct ValidationLog;
static ERRORS: AtomicUsize = AtomicUsize::new(0);
impl log::Log for ValidationLog {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Warn
    }
    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            eprintln!("{}", record.args());
            if record.level() == log::Level::Error {
                ERRORS.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    fn flush(&self) {}
}

#[test]
#[ignore = "requires a Vulkan GPU and validation layers"]
fn repeated_text_updates_reuse_slots_without_validation_errors() -> Result<()> {
    log::set_logger(&ValidationLog).unwrap();
    log::set_max_level(log::LevelFilter::Warn);
    let event_loop = EventLoopBuilder::new().with_any_thread(true).build()?;
    let window = WindowBuilder::new()
        .with_visible(false)
        .with_inner_size(winit::dpi::PhysicalSize::new(320, 240))
        .build(&event_loop)?;
    let mut renderer = unsafe { VulkanRenderer::create(&window)? };
    eprintln!("GPU: {}", unsafe {
        renderer
            .instance
            .get_physical_device_properties(renderer.data.physical_device)
            .device_name
    });
    let result = (|| -> Result<()> {
        let legacy = std::env::var_os("KANI_TEST_LEGACY_TEXT").is_some();
        let font = renderer.load_font(std::fs::read(
            std::env::var("KANI_TEST_FONT").unwrap_or_else(|_| "C:/Windows/Fonts/arial.ttf".into()),
        )?)?;
        renderer.data.camera.fov_y = 60.0;
        renderer.data.camera.near = 0.1;
        renderer.data.camera.far = 100.0;
        let mut mesh = GpuTextMesh::default();
        let mut samples = Vec::new();
        let mut warm_slots = 0;
        for frame in 0..600 {
            // Digits, empty text, capacity changes and UI/world pipelines.
            let content = match frame % 60 {
                0 => "".to_owned(),
                1 => "AMMO 18 / 18 0123456789 ABCDEFGHIJKLMNOPQRSTUVWXYZ".repeat(8),
                _ => format!("AMMO {} / 18", frame % 19),
            };
            let start = std::time::Instant::now();
            unsafe {
                let next = if legacy {
                    legacy_text(&mut renderer, font, &content)?
                } else {
                    renderer.create_text_mesh(font, &content, 20.0, 24.0)?
                };
                renderer.destroy_text_mesh(&mut mesh)?;
                mesh = next;
                renderer.set_text_render_items(
                    mesh.batches
                        .iter()
                        .map(|batch| TextRenderItem {
                            mesh: batch.mesh,
                            atlas_page: batch.page,
                            transform: Transform::default(),
                            alpha: 1.0,
                            color: [1.0; 4],
                            pipeline_key: if frame % 2 == 0 {
                                PipelineKey::TextUi2D
                            } else {
                                PipelineKey::Text3D
                            },
                        })
                        .collect(),
                );
                if frame >= 120 {
                    samples.push(start.elapsed().as_secs_f64() * 1000.0);
                }
                renderer.render(&window)?;
                if frame == 300 {
                    renderer.resized = true;
                }
            }
            if frame == 119 {
                warm_slots = renderer.data.meshes.len();
            }
        }
        anyhow::ensure!(
            legacy || renderer.data.meshes.len() == warm_slots,
            "mesh slots grew after warmup: {} -> {}",
            warm_slots,
            renderer.data.meshes.len()
        );
        unsafe {
            renderer.set_text_render_items(Vec::new());
            renderer.destroy_text_mesh(&mut mesh)?;
            for _ in 0..4 {
                renderer.render(&window)?;
            }
            for _ in 0..if legacy { 0 } else { 1000 } {
                let mut mesh = renderer.create_text_mesh(font, "AMMO 17 / 18", 20.0, 24.0)?;
                renderer.destroy_text_mesh(&mut mesh)?;
            }
        }
        anyhow::ensure!(
            legacy || renderer.data.meshes.len() == warm_slots,
            "unsubmitted updates grew mesh slots"
        );
        samples.sort_by(f64::total_cmp);
        eprintln!(
            "text update (CPU, validation enabled, legacy={}): p95={:.3}ms p99={:.3}ms max={:.3}ms slots={}",
            legacy,
            samples[samples.len() * 95 / 100],
            samples[samples.len() * 99 / 100],
            samples.last().unwrap(),
            renderer.data.meshes.len()
        );
        if !legacy {
            // Above the spare-cache budget: release the GPU allocation, then
            // reuse its management slot rather than growing the mesh table.
            let vertex = crate::vertex::Ui2DVertex::new(
                cgmath::vec2(0.0, 0.0),
                cgmath::vec3(1.0, 1.0, 1.0),
                cgmath::vec2(0.0, 0.0),
            );
            let vertices = vec![vertex; 600_000];
            let mut slot_count = 0;
            for pass in 0..2 {
                unsafe {
                    let handle = renderer.text_buffers.upload(
                        &renderer.instance,
                        &renderer.device,
                        &mut renderer.data,
                        &vertices,
                        &[0, 1, 2],
                    )?;
                    renderer.destroy_mesh(handle)?;
                    anyhow::ensure!(
                        renderer.data.meshes[handle.index].is_none(),
                        "oversized spare retained"
                    );
                }
                if pass == 0 {
                    slot_count = renderer.data.meshes.len();
                } else {
                    anyhow::ensure!(
                        renderer.data.meshes.len() == slot_count,
                        "freed slot not reused"
                    );
                }
            }
        }
        Ok(())
    })();
    unsafe {
        renderer.destroy();
    }
    result?;
    anyhow::ensure!(
        ERRORS.load(Ordering::Relaxed) == 0,
        "Vulkan validation reported errors"
    );
    Ok(())
}

// Reproduce the pre-fix mesh path solely for an A/B measurement. Production
// retains one path; this uses the unchanged ordinary mesh uploader/destructor.
unsafe fn legacy_text(
    renderer: &mut VulkanRenderer,
    font: crate::FontHandle,
    content: &str,
) -> Result<GpuTextMesh> {
    let buffer = renderer
        .text_system
        .layout_text(&renderer.fonts[font.0], content, 20.0, 24.0)?;
    let mesh = kani_volcano_text::build_text_mesh(
        &mut renderer.text_system,
        &mut renderer.glyph_atlas,
        &buffer,
    )?;
    renderer.upload_text_atlas()?;
    let mut result = GpuTextMesh::default();
    for batch in mesh.batches {
        if batch.vertices.is_empty() || batch.indices.is_empty() {
            continue;
        }
        let vertices = batch
            .vertices
            .iter()
            .map(|v| {
                crate::vertex::Ui2DVertex::new(
                    cgmath::vec2(v.position[0], v.position[1]),
                    cgmath::vec3(1.0, 1.0, 1.0),
                    cgmath::vec2(v.uv[0], v.uv[1]),
                )
            })
            .collect();
        let handle = renderer.load_mesh_from_data(
            crate::MeshData {
                vertices,
                indices: batch.indices,
            },
            crate::VertexLayout::Ui2D,
        )?;
        result.batches.push(crate::GpuTextBatch {
            page: batch.page,
            mesh: handle,
        });
    }
    Ok(result)
}
