use crate::core::{soundchunk::CHUNK_SIZE, uniqueid::UniqueId};

pub struct SoundInputTag;

pub type SoundInputId = UniqueId<SoundInputTag>;

// TODO: rename to (an)isochronous
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum InputOptions {
    Synchronous,
    NonSynchronous,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ReleaseStatus {
    NotYet,
    Pending { offset: usize },
    Released,
}

#[derive(Clone, Copy)]
pub struct InputTiming {
    sample_offset: usize,
    time_speed: f32,
    // TODO: add pending sample offset for starting over
    need_to_start_over: bool,
    is_done: bool,
    release: ReleaseStatus,
}

impl InputTiming {
    pub fn flag_to_start_over(&mut self) {
        self.need_to_start_over = true;
        self.is_done = false;
    }

    pub fn need_to_start_over(&self) -> bool {
        self.need_to_start_over
    }

    pub fn is_done(&self) -> bool {
        self.is_done
    }

    pub fn mark_as_done(&mut self) {
        self.is_done = true;
    }

    pub fn request_release(&mut self, sample_offset: usize) {
        if self.release == ReleaseStatus::Released {
            return;
        }
        debug_assert!(sample_offset < CHUNK_SIZE);
        self.release = ReleaseStatus::Pending {
            offset: sample_offset,
        };
    }

    pub fn pending_release(&self) -> Option<usize> {
        if let ReleaseStatus::Pending { offset } = self.release {
            Some(offset)
        } else {
            None
        }
    }

    pub fn take_pending_release(&mut self) -> Option<usize> {
        if let ReleaseStatus::Pending { offset } = self.release {
            self.release = ReleaseStatus::Released;
            Some(offset)
        } else {
            None
        }
    }

    pub fn was_released(&self) -> bool {
        self.release == ReleaseStatus::Released
    }

    pub fn start_over(&mut self, sample_offset: usize) {
        debug_assert!(sample_offset < CHUNK_SIZE);
        self.sample_offset = sample_offset;
        self.need_to_start_over = false;
        self.is_done = false;
        self.release = ReleaseStatus::NotYet;
    }

    pub fn sample_offset(&self) -> usize {
        self.sample_offset
    }

    pub fn time_speed(&self) -> f32 {
        self.time_speed
    }

    pub fn set_time_speed(&mut self, speed: f32) {
        assert!(speed >= 0.0);
        self.time_speed = speed;
    }
}

impl Default for InputTiming {
    fn default() -> InputTiming {
        InputTiming {
            sample_offset: 0,
            time_speed: 1.0,
            need_to_start_over: true,
            is_done: false,
            release: ReleaseStatus::NotYet,
        }
    }
}
