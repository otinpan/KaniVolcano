use anyhow::{Result, Context, anyhow};
use cgmath::{vec2, vec3};
use renderer_vulkan::{
    MeshHandle, SkyboxTextureHandle, TextureHandle, VertexLayout, VulkanRenderer,
    Msaa,
};
use kani_volcano_audio::{
    AudioSystem,
};
use winit::event::WindowEvent;
use winit::window::Window;

pub const DEFAULT_TEXTURE: TextureHandle = TextureHandle(0);
pub const DEFAULT_SKYBOX_TEXTURE: SkyboxTextureHandle = SkyboxTextureHandle(0);

use crate::primitive::{
    PrimitiveMesh, PrimitiveShape, build_primitive_source, create_primitive_debug_line,
    create_primitive_lit3d, create_primitive_mesh3d, create_primitive_ui2d,
};

use crate::{
    MeshAssetId, PipelineKey, Scene, SceneCommand, SceneCommandQueue, SceneContext, SceneId,
    UpdateContext, FontAssetId, AudioAssetId, TimeManager, TimeManagementApi, TimeManagementCommandQueue,
};

use super::{Input, Resources, SceneManager, Scheduler, Time, World};

pub type Vec3 = cgmath::Vector3<f32>;
pub type Vec2 = cgmath::Vector2<f32>;

pub struct App {
    pub renderer: VulkanRenderer,
    pub audio_system: AudioSystem,
    pub world: World,
    pub input: Input,
    pub time: Time,
    resources: Resources,

    // system
    scheduler: Scheduler,
    scene_manager: SceneManager,
    scene_commands: SceneCommandQueue,

    // timer
    fixed_accumulator: f64,
    fixed_time: Time,
    fixed_delta: f64,
    time_manager: TimeManager,
    simulation_tick: u64,
    game_input: Input,
}

impl TimeManagementApi for App {
    fn time_management_commands_mut(&mut self) -> &mut TimeManagementCommandQueue {
        &mut self.scheduler.time_management_commands
    }
}

fn consume_simulation_tick(
    manager: &mut TimeManager,
    accumulator: &mut f64,
    fixed_delta: f64,
) -> bool {
    if manager.is_paused() {
        *accumulator = 0.0;
        manager.consume_step()
    } else if *accumulator >= fixed_delta {
        *accumulator -= fixed_delta;
        true
    } else {
        false
    }
}

impl App {
    pub unsafe fn create(window: &Window) -> Result<Self> {
        let audio_system=AudioSystem::new()?;
        let mut renderer = VulkanRenderer::create(window)?;
        let world = World::default();

        // load data
        let mut resources = Resources::default();

        // Register the backend master bus by name.
        resources.register_audio_bus(
            "master",
            audio_system.master_bus(),
        )?;

        let primitive_meshes = create_primitive_meshes(&mut renderer, &mut resources)?;
        resources.set_primitive_meshes(primitive_meshes);

        let mut input = Input::default();
        let window_size = window.inner_size();
        input.set_window_size(vec2(window_size.width as f32, window_size.height as f32));

        let scheduler = Scheduler::default();
        let scene_manager = SceneManager::default();
        let scene_commands = SceneCommandQueue::default();
        let mut app = Self {
            renderer,
            audio_system,
            world,
            input,
            time: Time::default(),
            resources,
            scheduler,
            scene_manager,
            scene_commands,
            fixed_accumulator: 0.0,
            fixed_time: Time::default(),
            fixed_delta: 1.0 / 60.0,
            time_manager: TimeManager::default(),
            simulation_tick: 0,
            game_input: Input::default(),
        };

        app.scheduler
            .run_render_stage(&mut app.world, &mut app.renderer, &mut app.resources)?;

        Ok(app)
    }

    pub fn handle_event(&mut self, event: &WindowEvent) {
        self.input.handle_event(event);
    }

    pub unsafe fn render(&mut self, window: &Window) -> Result<()> {
        self.renderer.render(window)
    }

    pub fn update(&mut self) -> Result<()> {
        self.time.update();
        let delta_time = self.time.delta_seconds();


        // register assets in resources and renderer in main thread
        let outcomes=unsafe{
            self.scheduler.register_asset_stage(
                &mut self.renderer,
                &mut self.audio_system,
                &mut self.resources,
            )?
        };

        for outcome in outcomes{
            if let Err(error) = outcome.result{
                log::error!(
                    "asset load failed: {:?} '{}': {:#}",
                    outcome.kind,
                    outcome.name,
                    error,
                );
            }
        }

        self.game_input.accumulate_frame(&self.input);
        let mut commands = self.scheduler.run_debug_input_stage(&self.input);

        self.scheduler.run_command_stage(
            &mut commands,
            &mut self.world,
            &self.input,
            &mut self.resources,
            &mut self.scene_commands,
        )?;

        let control_changed = self.apply_time_management_commands();
        // Discard wall time on control changes and while paused.
        if !control_changed && !self.time_manager.is_paused() {
            self.fixed_accumulator += f64::from(delta_time).min(0.25);
        }

        // Large Step requests are spread across frames so controls stay responsive.
        for _ in 0..16 {
            self.apply_time_management_commands();
            if !consume_simulation_tick(
                &mut self.time_manager,
                &mut self.fixed_accumulator,
                self.fixed_delta,
            ) {
                break;
            }
            self.run_game_tick()?;
            self.game_input.clear_transitions();
        }
        self.apply_time_management_commands();
        if self.time_manager.is_paused() {
            self.game_input.clear_transitions();
        }

        // Frame updates use wall time and remain active while paused.
        self.scheduler.run_update_stage(
            &mut self.world,
            &self.input,
            &self.time,
            &mut self.resources,
            &mut self.scene_commands,
        )?;

        // Debug commands may request a scene change even while paused.
        self.apply_scene_commands()?;

        // load asset (read file, create pixels) in worker thread (maximum 4)
        self.scheduler.load_asset_stage()?;

        self.scheduler.run_audio_stage(
            &mut self.audio_system,
            &mut self.resources,
        )?;

        self.scheduler.run_render_stage(
            &mut self.world,
            &mut self.renderer,
            &mut self.resources,
        )?;

        self.input.clear_transitions();
        Ok(())
    }

    fn apply_time_management_commands(&mut self) -> bool {
        if self.scheduler.time_management_commands.is_empty() {
            return false;
        }
        self.time_manager.apply(&mut self.scheduler.time_management_commands);
        self.fixed_accumulator = 0.0;
        true
    }

    fn run_game_tick(&mut self) -> Result<()> {
        self.fixed_time.advance_fixed(self.fixed_delta as f32);
        let mut commands = self.scheduler.run_input_stage(&self.game_input);
        self.scheduler.run_command_stage(
            &mut commands,
            &mut self.world,
            &self.game_input,
            &mut self.resources,
            &mut self.scene_commands,
        )?;
        self.scheduler.run_fixed_update_stage(
            &mut self.world,
            &self.game_input,
            &self.fixed_time,
            &mut self.resources,
            &mut self.scene_commands,
        )?;
        let mut context = UpdateContext::new(
            &mut self.world,
            &self.game_input,
            &self.fixed_time,
            &mut self.resources,
            &mut self.scheduler.render_commands,
            &mut self.scheduler.asset_load_commands,
            &mut self.scheduler.audio_commands,
            &mut self.scene_commands,
            &mut self.scheduler.time_management_commands,
        );
        self.scene_manager.update_current_scene(&mut context)?;
        self.apply_scene_commands()?;
        self.simulation_tick += 1;
        Ok(())
    }

    pub fn is_paused(&self) -> bool {
        self.time_manager.is_paused()
    }

    pub fn simulation_tick(&self) -> u64 {
        self.simulation_tick
    }

    pub fn simulation_time(&self) -> &Time {
        &self.fixed_time
    }

    pub unsafe fn destroy(&mut self) {
        self.renderer.destroy();
    }

    pub fn add_scene<S>(&mut self, scene: S) -> Result<SceneId>
    where
        S: Scene + 'static,
    {
        self.scene_manager.add_scene(scene)
    }

    pub fn set_current_scene(&mut self, name: &str) -> Result<SceneId> {
        if self.scene_manager.current_scene_name() == Some(name) {
            return Ok(self
                .scene_manager
                .current_scene_id()
                .expect("current scene should have a scene id"));
        }

        if let Some(current_scene_id) = self.scene_manager.current_scene_id() {
            let mut context = SceneContext::new(
                current_scene_id,
                &mut self.world,
                &self.input,
                &self.time,
                &mut self.resources,
                &mut self.scheduler,
            );

            self.scene_manager.exit_current_scene(&mut context)?;
            self.scheduler.reset_scene_runtime();
        }

        let scene_id = self.scene_manager.set_current_scene(name)?;
        let mut context = SceneContext::new(
            scene_id,
            &mut self.world,
            &self.input,
            &self.time,
            &mut self.resources,
            &mut self.scheduler,
        );

        self.scene_manager.enter_current_scene(&mut context)?;

        Ok(scene_id)
    }

    pub unsafe fn load_model(
        &mut self,
        name: &str,
        path: &str,
        pipeline_key: PipelineKey,
        auto_release: bool,
    ) -> Result<MeshAssetId> {
        let vertex_layout = pipeline_key.required_vertex_layout();

        let handle = match vertex_layout {
            VertexLayout::Mesh3D => self.renderer.load_mesh3d_from_model(path)?,
            VertexLayout::DebugLine3D => self.renderer.load_debug_line_from_model(path)?,
            VertexLayout::Lit3D => self.renderer.load_lit3d_from_model(path)?,
            VertexLayout::Ui2D => {
                return Err(anyhow!("Ui2D model loading is not supported"));
            }
            VertexLayout::Skybox => {
                return Err(anyhow!("Skybox model loading is not supported"));
            }
        };

        Ok(self.resources.register_model(name, handle, auto_release))
    }

    pub unsafe fn load_texture(&mut self, name: &str, path: &str) -> Result<TextureHandle> {
        let handle = self.renderer.load_texture(path)?;

        Ok(self.resources.register_texture(name, handle))
    }

    pub fn create_skybox(&mut self, half_size: f32) -> Result<()> {
        unsafe {
            let mesh = create_skybox_mesh(&mut self.renderer, half_size)?;
            self.resources.set_skybox_mesh(mesh);
        }
        Ok(())
    }

    pub unsafe fn load_skybox_texture(
        &mut self,
        name: &str,
        paths: [&str; 6],
    ) -> Result<SkyboxTextureHandle> {
        let handle = self.renderer.load_skybox_texture(paths)?;
        Ok(self.resources.register_skybox_texture(name, handle))
    }

    pub unsafe fn load_font(
        &mut self,
        name: &str,
        path: &str,
    ) -> Result<FontAssetId> {
        anyhow::ensure!(
            self.resources.font_asset_id(name).is_none(),
            "font already registered: {name}"
        );

        let bytes=std::fs::read(path)
            .with_context(|| format!("failed to read font file: {path}"))?;

        let handle=self.renderer.load_font(bytes)?;
        self.resources.register_font(name,handle)
    }

    pub unsafe fn load_audio(
        &mut self,
        name: &str,
        path: &str,
    ) -> Result<AudioAssetId>{
        anyhow::ensure!(
            self.resources.audio_asset_id(name).is_none(),
            "audio already registered: {name}"
        );

        let handle=self.audio_system.load_audio(path)?;
        self.resources.register_audio(name, handle)
    }

    /// Sets the fixed update interval in seconds.
    /// Returns an error for non-finite or non-positive intervals.
    pub fn set_fixed_delta_time(&mut self, fixed_delta_time: f32) -> Result<()> {
        anyhow::ensure!(
            fixed_delta_time.is_finite() && fixed_delta_time > 0.0,
            "fixed delta time must be finite and positive"
        );
        self.fixed_delta = f64::from(fixed_delta_time);
        Ok(())
    }

    fn apply_scene_commands(&mut self) -> Result<()> {
        let commands = self.scene_commands.drain().collect::<Vec<_>>();

        for command in commands {
            match command {
                SceneCommand::SetCurrentScene { scene_name } => {
                    self.set_current_scene(&scene_name)?;
                }
            }
        }
        Ok(())
    }

    /// Returns the currently applied MSAA setting, excluding queued changes.
    pub fn msaa(&self) -> Msaa {
        self.renderer.msaa()
    }

    /// Queues an MSAA change for the next render stage.
    /// Unsupported settings are reported when the command is executed.
    pub fn set_msaa(&mut self, msaa: Msaa) {
        self.scheduler.render_commands.set_msaa(msaa);
    }

}

unsafe fn create_skybox_mesh(renderer: &mut VulkanRenderer, size: f32) -> Result<MeshHandle> {
    let h = size;
    let source = build_primitive_source(PrimitiveShape::Cube {
        points: [
            vec3(h, -h, h),
            vec3(h, h, h),
            vec3(-h, h, h),
            vec3(-h, -h, h),
            vec3(h, -h, -h),
            vec3(h, h, -h),
            vec3(-h, h, -h),
            vec3(-h, -h, -h),
        ],
        color: vec3(1.0, 1.0, 1.0),
    });

    renderer.load_mesh_from_data(source.to_skybox_data(), VertexLayout::Skybox)
}

// these primitives are not released (resources.release_mesh) by DespawnLastCommand
// becuase DespawnLastCommand release only Entitis that have MeshRenderer Component
unsafe fn create_primitive_meshes(
    renderer: &mut VulkanRenderer,
    resources: &mut Resources,
) -> Result<Vec<PrimitiveMesh>> {
    let primitive_meshes = vec![
        create_primitive_lit3d(
            renderer,
            resources,
            PrimitiveShape::Triangle {
                points: [
                    vec3(0.0, 0.0, 0.5),
                    vec3(0.0, -0.5, -0.5),
                    vec3(0.0, 0.5, -0.5),
                ],
                color: vec3(1.0, 1.0, 1.0),
            },
            false,
        )?,
        create_primitive_ui2d(
            renderer,
            resources,
            PrimitiveShape::Rectangle {
                points: [
                    vec3(0.0, -0.5, 0.5),
                    vec3(0.0, -0.5, -0.5),
                    vec3(0.0, 0.5, -0.5),
                    vec3(0.0, 0.5, 0.5),
                ],
                color: vec3(1.0, 1.0, 0.0),
            },
            false,
        )?,
        create_primitive_debug_line(
            renderer,
            resources,
            PrimitiveShape::Cube {
                points: [
                    vec3(0.5, -0.5, 0.5),
                    vec3(0.5, 0.5, 0.5),
                    vec3(-0.5, 0.5, 0.5),
                    vec3(-0.5, -0.5, 0.5),
                    vec3(0.5, -0.5, -0.5),
                    vec3(0.5, 0.5, -0.5),
                    vec3(-0.5, 0.5, -0.5),
                    vec3(-0.5, -0.5, -0.5),
                ],
                color: vec3(1.0, 0.0, 0.0),
            },
            false,
        )?,
        create_primitive_mesh3d(
            renderer,
            resources,
            PrimitiveShape::Circle {
                radius: 1.0,
                segments: 32,
                color: vec3(0.0, 1.0, 1.0),
            },
            false,
        )?,
        create_primitive_mesh3d(
            renderer,
            resources,
            PrimitiveShape::Polygon {
                points: vec![
                    vec3(0.0, -0.7, 0.7),
                    vec3(0.0, -0.4, 0.5),
                    vec3(0.0, 0.7, 0.5),
                    vec3(0.0, 0.0, -0.6),
                    vec3(0.0, -0.5, -0.4),
                ],
                color: vec3(0.0, 1.0, 0.0),
            },
            false,
        )?,
        create_primitive_lit3d(
            renderer,
            resources,
            PrimitiveShape::Sphere {
                radius: 1.0,
                rings: 32,
                segments: 32,
                color: vec3(0.0, 0.0, 1.0),
            },
            false,
        )?,
    ];

    Ok(primitive_meshes)
}

#[cfg(test)]
mod simulation_tests {
    use super::*;
    use crate::TimeManagementCommand;

    #[test]
    fn paused_wall_time_is_discarded_and_resume_does_not_catch_up() {
        let mut manager = TimeManager::default();
        let mut queue = TimeManagementCommandQueue::default();
        queue.push(TimeManagementCommand::Pause);
        manager.apply(&mut queue);
        let mut accumulator = 120.0;
        assert!(!consume_simulation_tick(&mut manager, &mut accumulator, 0.02));
        assert_eq!(accumulator, 0.0);
        queue.push(TimeManagementCommand::Resume);
        manager.apply(&mut queue);
        assert!(!consume_simulation_tick(&mut manager, &mut accumulator, 0.02));
        accumulator += 0.02;
        assert!(consume_simulation_tick(&mut manager, &mut accumulator, 0.02));
    }

    #[test]
    fn step_advances_exactly_requested_ticks_with_fixed_time() {
        let mut manager = TimeManager::default();
        let mut queue = TimeManagementCommandQueue::default();
        queue.push(TimeManagementCommand::Step(3));
        manager.apply(&mut queue);
        let mut time = Time::default();
        let mut accumulator = 100.0;
        for _ in 0..3 {
            assert!(consume_simulation_tick(&mut manager, &mut accumulator, 0.02));
            time.advance_fixed(0.02);
        }
        assert!(!consume_simulation_tick(&mut manager, &mut accumulator, 0.02));
        assert!(manager.is_paused());
        assert!((time.elapsed_seconds() - 0.06).abs() < 1e-6);
    }

    #[test]
    fn pause_from_a_tick_cancels_remaining_steps_at_the_boundary() {
        let mut manager = TimeManager::default();
        let mut queue = TimeManagementCommandQueue::default();
        queue.push(TimeManagementCommand::Step(10));
        manager.apply(&mut queue);
        let mut accumulator = 0.0;
        assert!(consume_simulation_tick(&mut manager, &mut accumulator, 0.02));
        queue.push(TimeManagementCommand::Pause);
        manager.apply(&mut queue);
        assert!(!consume_simulation_tick(&mut manager, &mut accumulator, 0.02));
        assert!(queue.is_empty());
    }
}
