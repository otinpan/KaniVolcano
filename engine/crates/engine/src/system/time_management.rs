#[derive(Debug)]
pub enum TimeManagementCommand{
    Pause,
    Resume,
    TogglePause,
    Step(u32),
}

#[derive(Default)]
pub struct TimeManagementCommandQueue{
    commands: Vec<TimeManagementCommand>,
}

impl TimeManagementCommandQueue{
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub(crate) fn drain(&mut self) -> impl Iterator<Item=TimeManagementCommand> +'_{
        self.commands.drain(..)
    }

    pub fn push(&mut self, command: TimeManagementCommand){
        self.commands.push(command);
    }
}


#[derive(Default)]
pub struct TimeManager{
    paused: bool,
    pending_steps: u32,
}

impl TimeManager{
    pub fn is_paused(&self) -> bool{
        self.paused
    }

    pub fn apply(&mut self, queue: &mut TimeManagementCommandQueue){
        for command in queue.commands.drain(..){
            match command{
                TimeManagementCommand::Pause=>{
                    self.paused=true;
                    self.pending_steps=0;
                }
                TimeManagementCommand::Resume=>{
                    self.paused=false;
                    self.pending_steps=0;
                }
                TimeManagementCommand::TogglePause => {
                    self.paused = !self.paused;
                    self.pending_steps = 0;
                }
                TimeManagementCommand::Step(count) =>{
                    self.paused=true;
                    self.pending_steps=self.pending_steps.saturating_add(count);
                }
            }
        }
    }

    pub fn consume_step(&mut self) -> bool{
        if self.paused && self.pending_steps>0{
            self.pending_steps-=1;
            true
        }else{
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_uses_current_state_and_cancels_pending_steps() {
        let mut manager = TimeManager::default();
        let mut queue = TimeManagementCommandQueue::default();
        queue.push(TimeManagementCommand::TogglePause);
        manager.apply(&mut queue);
        assert!(manager.is_paused());

        queue.push(TimeManagementCommand::Step(3));
        queue.push(TimeManagementCommand::TogglePause);
        manager.apply(&mut queue);
        assert!(!manager.is_paused());

        queue.push(TimeManagementCommand::TogglePause);
        manager.apply(&mut queue);
        assert!(manager.is_paused());
        assert!(!manager.consume_step());
        assert!(queue.is_empty());
    }
}
