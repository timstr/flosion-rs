use crate::core::soundchunk::CHUNK_SIZE;

use super::uniqueid::UniqueId;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SoundInputId(pub usize);

impl Default for SoundInputId {
    fn default() -> SoundInputId {
        SoundInputId(1)
    }
}

impl UniqueId for SoundInputId {
    fn value(&self) -> usize {
        self.0
    }
    fn next(&self) -> SoundInputId {
        SoundInputId(self.0 + 1)
    }
}

#[derive(Copy, Clone)]
pub struct InputOptions {
    // Will the input ever be paused or reset by the sound processor?
    pub interruptible: bool,

    // Will the input's speed of time always be the same as the sound processor's?
    pub realtime: bool,
}

#[derive(Clone, Copy)]
pub struct InputTiming {
    sample_offset: usize,
    // TODO: add pending sample offset for resetting
    needs_reset: bool,
    is_done: bool,
}

impl InputTiming {
    pub fn require_reset(&mut self) {
        self.needs_reset = true;
    }

    pub fn needs_reset(&self) -> bool {
        self.needs_reset
    }

    pub fn is_done(&self) -> bool {
        self.is_done
    }

    pub fn mark_as_done(&mut self) {
        self.is_done = true;
    }

    pub fn reset(&mut self, sample_offset: usize) {
        debug_assert!(sample_offset < CHUNK_SIZE);
        self.sample_offset = sample_offset;
        self.needs_reset = false;
        self.is_done = false;
    }

    pub fn sample_offset(&self) -> usize {
        self.sample_offset
    }
}

impl Default for InputTiming {
    fn default() -> InputTiming {
        InputTiming {
            sample_offset: 0,
            needs_reset: true,
            is_done: false,
        }
    }
}
