use crate::core::soundchunk::CHUNK_SIZE;

use super::{
    context::Context,
    soundchunk::SoundChunk,
    soundprocessor::{ProcessorState, StreamStatus},
    statetree::{AnyData, ProcessorNodeWrapper},
    uniqueid::UniqueId,
};

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
    // Will the input's speed of time always be the same as the sound processor's?
    pub realtime: bool,
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
    // TODO: add pending sample offset for resetting
    needs_reset: bool,
    is_done: bool,
    release: ReleaseStatus,
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

    pub fn request_release(&mut self, sample_offset: usize) {
        debug_assert!(self.release == ReleaseStatus::NotYet);
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
        self.release != ReleaseStatus::NotYet
    }

    pub fn reset(&mut self, sample_offset: usize) {
        debug_assert!(sample_offset < CHUNK_SIZE);
        self.sample_offset = sample_offset;
        self.needs_reset = false;
        self.is_done = false;
        self.release = ReleaseStatus::NotYet;
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
            release: ReleaseStatus::NotYet,
        }
    }
}

// TODO: move this to a dedicated struct that
// 1. wraps the target (Option<Box<dyn ProcessorNodeWrapper>>) into a clean interface
// 2. provides easy access for the upcoming node allocation whatever to modify the
//    target in place
// Maybe call it SoundInputTarget?
// Have each SoundInputNode implementation store one of these for each
pub(super) fn step_sound_input<T: ProcessorState>(
    timing: &mut InputTiming,
    target: &mut Option<Box<dyn ProcessorNodeWrapper>>,
    state: &T,
    dst: &mut SoundChunk,
    ctx: &Context,
    input_state: AnyData<SoundInputId>,
) -> StreamStatus {
    debug_assert!(!timing.needs_reset());
    if timing.is_done() {
        dst.silence();
        return StreamStatus::Done;
    }
    if let Some(node) = target {
        let ctx = ctx.push_processor_state(state);
        let ctx = ctx.push_input(Some(node.id()), input_state, timing);
        let status = node.process_audio(dst, ctx);
        if status == StreamStatus::Done {
            debug_assert!(!timing.is_done());
            timing.mark_as_done();
        }
        status
    } else {
        timing.mark_as_done();
        dst.silence();
        StreamStatus::Done
    }
}
