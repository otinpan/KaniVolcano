use std::time::Instant;

#[derive(Debug)]
pub struct Time {
    start: Instant,
    previous_frame: Instant,
    delta_seconds: f32,
    elapsed_seconds: f32,
}

impl Time {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start: now,
            previous_frame: now,
            delta_seconds: 0.0,
            elapsed_seconds: 0.0,
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        self.delta_seconds = (now - self.previous_frame).as_secs_f32();
        self.elapsed_seconds = (now - self.start).as_secs_f32();
        self.previous_frame = now;
    }

    pub fn delta_seconds(&self) -> f32 {
        self.delta_seconds
    }

    pub fn elapsed_seconds(&self) -> f32 {
        self.elapsed_seconds
    }

    pub(crate) fn advance_fixed(&mut self, delta_seconds: f32){
        self.delta_seconds=delta_seconds;
        self.elapsed_seconds+=delta_seconds;
    }
}

impl Default for Time {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn new_time_starts_at_zero() {
        let time = Time::new();

        assert_eq!(time.delta_seconds(), 0.0);
        assert_eq!(time.elapsed_seconds(), 0.0);
    }

    #[test]
    fn update_advances_delta_and_elapsed_time() {
        let mut time = Time::new();

        thread::sleep(Duration::from_millis(1));
        time.update();

        assert!(time.delta_seconds() >= 0.0);
        assert!(time.elapsed_seconds() >= time.delta_seconds());
    }
}
