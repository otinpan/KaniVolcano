use crate::{TimeManagementCommand, TimeManagementCommandQueue};

/// Queues simulation control requests for application at a tick boundary.
/// Calling these methods does not interrupt the currently running system.
pub trait TimeManagementApi {
    fn time_management_commands_mut(&mut self) -> &mut TimeManagementCommandQueue;

    /// Requests a pause and cancels any pending steps.
    fn pause(&mut self) {
        self.time_management_commands_mut()
            .push(TimeManagementCommand::Pause);
    }

    /// Requests normal execution and cancels any pending steps.
    fn resume(&mut self) {
        self.time_management_commands_mut()
            .push(TimeManagementCommand::Resume);
    }

    /// Requests toggling the current pause state and cancels pending steps.
    fn toggle_pause(&mut self) {
        self.time_management_commands_mut()
            .push(TimeManagementCommand::TogglePause);
    }

    /// Requests a pause and adds `ticks` to the pending fixed-update steps.
    /// Zero ticks requests a pause without adding steps.
    fn step(&mut self, ticks: u32) {
        self.time_management_commands_mut()
            .push(TimeManagementCommand::Step(ticks));
    }
}
