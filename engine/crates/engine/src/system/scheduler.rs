use anyhow::Result;

use super::{
    Command, CommandContext, CommandQueue, CommandSystem, InputSystem, InputTrigger,
    RenderCommandQueue, RenderContext, RenderSystem, ScheduledUpdateSystem, UpdateContext,
    UpdateSystem,
};

use crate::{Input, Resources, SceneCommandQueue, Time, World};
use cgmath::Vector3;
use renderer_vulkan::VulkanRenderer;

pub type Vec3 = Vector3<f32>;

pub struct Scheduler {
    pub command_system: CommandSystem,
    pub input_system: InputSystem,
    pub render_system: RenderSystem,
    pub render_commands: RenderCommandQueue,
    update_systems: Vec<ScheduledUpdateSystem>,
    fixed_update_systems: Vec<ScheduledUpdateSystem>,
}

impl Scheduler {
    pub fn with_input_system(input_system: InputSystem) -> Self {
        Self {
            input_system,
            ..Default::default()
        }
    }

    pub fn new(
        command_system: CommandSystem,
        input_system: InputSystem,
        render_system: RenderSystem,
        render_commands: RenderCommandQueue,
        update_systems: Vec<ScheduledUpdateSystem>,
        fixed_update_systems: Vec<ScheduledUpdateSystem>,
    ) -> Self {
        Self {
            command_system,
            input_system,
            render_system,
            render_commands,
            update_systems,
            fixed_update_systems,
        }
    }

    pub fn add_update_system<S>(&mut self, name: &str, system: S)
    where
        S: UpdateSystem + 'static,
    {
        if self.update_systems.iter().any(|s| s.name == name) {
            return;
        }

        self.update_systems.push(ScheduledUpdateSystem {
            name: name.to_string(),
            system: Box::new(system),
            enabled: true,
        });
    }

    pub fn add_fixed_update_system<S>(&mut self, name: &str, system: S)
    where 
        S: UpdateSystem + 'static,
    {
        if self.fixed_update_systems.iter().any(|s| s.name==name){
            return;
        }

        self.fixed_update_systems.push(ScheduledUpdateSystem{
            name: name.to_string(),
            system: Box::new(system),
            enabled: true,
        })
    }

    pub fn remove_update_system(&mut self, name: &str) -> bool {
        let before_len = self.update_systems.len();
        self.update_systems
            .retain(|scheduled_system| scheduled_system.name != name);

        self.update_systems.len() != before_len
    }

    pub fn reset_scene_runtime(&mut self) {
        self.input_system.reset();
        self.update_systems.clear();
    }

    pub fn bind_key<C>(&mut self, key: winit::keyboard::KeyCode, trigger: InputTrigger, command: C)
    where
        C: Command + 'static,
    {
        self.input_system.bind_in_place(key, trigger, command);
    }

    pub fn run_input_stage(&self, input: &Input) -> CommandQueue {
        self.input_system.update(input)
    }

    pub fn run_command_stage(
        &mut self,
        commands: &mut CommandQueue,
        world: &mut World,
        input: &Input,
        resources: &mut Resources,
        scene_commands: &mut SceneCommandQueue,
    ) -> Result<()> {
        let mut context = CommandContext::new(
            commands,
            world,
            input,
            resources,
            &mut self.render_commands,
            scene_commands,
        );

        self.command_system.update(&mut context)
    }

    pub fn run_update_stage(
        &mut self,
        world: &mut World,
        input: &Input,
        time: &Time,
        resources: &mut Resources,
        scene_commands: &mut SceneCommandQueue,
    ) -> Result<()> {
        let mut context = UpdateContext::new(
            world,
            input,
            time,
            resources,
            &mut self.render_commands,
            scene_commands,
        );

        for scheduled_system in &mut self.update_systems {
            if scheduled_system.enabled {
                scheduled_system.system.update(&mut context)?
            }
        }
        Ok(())
    }

    pub fn run_fixed_update_stage(
        &mut self,
        world: &mut World,
        input: &Input,
        time: &Time,
        resources: &mut Resources,
        scene_commands: &mut SceneCommandQueue,
    ) -> Result<()>{
        let mut context = UpdateContext::new(
            world,
            input,
            time,
            resources,
            &mut self.render_commands,
            scene_commands,
        );

        for scheduled_system in &mut self.fixed_update_systems {
            if scheduled_system.enabled {
                scheduled_system.system.update(&mut context)?
            }
        }
        Ok(())
    }

    pub fn run_render_stage(
        &mut self,
        world: &mut World,
        renderer: &mut VulkanRenderer,
        resources: &mut Resources,
    ) -> Result<()> {
        let mut context = RenderContext::new(world, resources, renderer, &mut self.render_commands);

        self.render_system.update(&mut context)
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self {
            command_system: CommandSystem,
            input_system: InputSystem::new(),
            render_system: RenderSystem,
            render_commands: RenderCommandQueue::default(),
            update_systems: Vec::new(),
            fixed_update_systems: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestUpdateSystem;

    impl UpdateSystem for TestUpdateSystem {
        fn update(&mut self, _context: &mut UpdateContext<'_>) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn remove_update_system_removes_system_by_name() {
        let mut scheduler = Scheduler::default();
        scheduler.add_update_system("first", TestUpdateSystem);
        scheduler.add_update_system("second", TestUpdateSystem);

        assert_eq!(scheduler.update_systems.len(), 2);
        assert!(scheduler.remove_update_system("first"));
        assert_eq!(scheduler.update_systems.len(), 1);
        assert_eq!(scheduler.update_systems[0].name, "second");
    }

    #[test]
    fn remove_update_system_returns_false_when_missing() {
        let mut scheduler = Scheduler::default();
        scheduler.add_update_system("first", TestUpdateSystem);

        assert!(!scheduler.remove_update_system("missing"));
        assert_eq!(scheduler.update_systems.len(), 1);
        assert_eq!(scheduler.update_systems[0].name, "first");
    }
}
