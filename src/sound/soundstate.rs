pub struct StateTime {
    elapsed_samples: usize,
    relative_time_speed: f32,
}

impl StateTime {
    pub fn new() -> StateTime {
        StateTime {
            elapsed_samples: 0,
            relative_time_speed: 1.0,
        }
    }

    pub fn reset(&mut self) {
        self.elapsed_samples = 0;
        self.relative_time_speed = 1.0;
    }
}

pub trait SoundState: Default + Send {
    fn reset(&mut self);
    fn time(&self) -> &StateTime;
    fn time_mut(&mut self) -> &mut StateTime;
}

pub struct EmptyState {
    time: StateTime,
}

impl Default for EmptyState {
    fn default() -> EmptyState {
        EmptyState {
            time: StateTime::new(),
        }
    }
}

impl SoundState for EmptyState {
    fn reset(&mut self) {}
    fn time(&self) -> &StateTime {
        &self.time
    }
    fn time_mut(&mut self) -> &mut StateTime {
        &mut self.time
    }
}
