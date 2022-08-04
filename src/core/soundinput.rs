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
    elapsed_chunks: usize,
    sample_offset: usize,
    needs_reset: bool,
}

impl InputTiming {
    pub fn require_reset(&mut self) {
        self.needs_reset = true;
    }

    pub fn needs_reset(&self) -> bool {
        self.needs_reset
    }

    pub fn advance_one_chunk(&mut self) -> () {
        debug_assert!(!self.needs_reset);
        self.elapsed_chunks += 1;
    }

    pub fn reset(&mut self, sample_offset: usize) {
        debug_assert!(sample_offset < CHUNK_SIZE);
        self.elapsed_chunks = 0;
        self.sample_offset = sample_offset;
        self.needs_reset = false;
    }

    pub fn elapsed_chunks(&self) -> usize {
        self.elapsed_chunks
    }

    pub fn sample_offset(&self) -> usize {
        self.sample_offset
    }

    pub fn total_samples(&self) -> usize {
        self.elapsed_chunks * CHUNK_SIZE + self.sample_offset
    }
}

impl Default for InputTiming {
    fn default() -> InputTiming {
        InputTiming {
            elapsed_chunks: 0,
            sample_offset: 0,
            needs_reset: true,
        }
    }
}
