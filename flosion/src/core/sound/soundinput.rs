use hashstash::{
    InplaceUnstasher, Stashable, Stasher, UnstashError, Unstashable, UnstashableInplace, Unstasher,
};

use crate::core::{
    engine::{
        soundgraphcompiler::SoundGraphCompiler,
        stategraphnode::{CompiledSoundInputBranch, StateGraphNodeValue},
    },
    jit::argumentstack::ArgumentStackView,
    soundchunk::CHUNK_SIZE,
    stashing::{StashingContext, UnstashingContext},
    uniqueid::UniqueId,
};

use super::{
    argument::{ArgumentScope, ArgumentTranslation, CompiledProcessorArgument},
    context::AudioContext,
    soundprocessor::SoundProcessorId,
};

pub struct ProcessorInputTag;

pub type ProcessorInputId = UniqueId<ProcessorInputTag>;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct SoundInputLocation {
    processor: SoundProcessorId,
    input: ProcessorInputId,
}

impl SoundInputLocation {
    pub(crate) fn new(processor: SoundProcessorId, input: ProcessorInputId) -> SoundInputLocation {
        SoundInputLocation { processor, input }
    }

    pub(crate) fn processor(&self) -> SoundProcessorId {
        self.processor
    }

    pub(crate) fn input(&self) -> ProcessorInputId {
        self.input
    }
}

impl Stashable for SoundInputLocation {
    fn stash(&self, stasher: &mut Stasher) {
        self.processor.stash(stasher);
        self.input.stash(stasher);
    }
}

impl Unstashable for SoundInputLocation {
    fn unstash(unstasher: &mut Unstasher) -> Result<Self, UnstashError> {
        Ok(SoundInputLocation {
            processor: SoundProcessorId::unstash(unstasher)?,
            input: ProcessorInputId::unstash(unstasher)?,
        })
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum InputBranching {
    Single,
    Multiple(usize),
}

// TODO: rename to (an)isochronous
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
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

#[derive(Eq, PartialEq, Debug)]
pub struct BasicProcessorInput {
    id: ProcessorInputId,
    options: InputOptions,
    branches: usize,
    target: Option<SoundProcessorId>,
    argument_scope: ArgumentScope,
}

impl BasicProcessorInput {
    pub fn new(
        options: InputOptions,
        branches: usize,
        argument_scope: ArgumentScope,
    ) -> BasicProcessorInput {
        BasicProcessorInput {
            id: ProcessorInputId::new_unique(),
            options,
            branches,
            target: None,
            argument_scope,
        }
    }

    pub(crate) fn id(&self) -> ProcessorInputId {
        self.id
    }

    pub(crate) fn options(&self) -> InputOptions {
        self.options
    }

    pub(crate) fn branches(&self) -> usize {
        self.branches
    }

    pub(crate) fn set_branches(&mut self, branches: usize) {
        self.branches = branches;
    }

    pub(crate) fn target(&self) -> Option<SoundProcessorId> {
        self.target
    }

    pub(crate) fn set_target(&mut self, target: Option<SoundProcessorId>) {
        self.target = target;
    }

    pub(crate) fn argument_scope(&self) -> &ArgumentScope {
        &self.argument_scope
    }

    pub fn compile_branch<'ctx>(
        &self,
        processor_id: SoundProcessorId,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> CompiledSoundInputBranch<'ctx> {
        let target = match self.target {
            Some(target_spid) => compiler.compile_sound_processor(target_spid),
            None => StateGraphNodeValue::Empty,
        };
        let location = SoundInputLocation::new(processor_id, self.id);
        CompiledSoundInputBranch::new(location, target)
    }
}

impl Stashable<StashingContext> for BasicProcessorInput {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.u64(self.id.value() as _);
        stasher.u8(match self.options {
            InputOptions::Synchronous => 0,
            InputOptions::NonSynchronous => 1,
        });
        stasher.u64(self.branches as _);
        match self.target {
            Some(spid) => {
                stasher.u8(1);
                stasher.u64(spid.value() as _);
            }
            None => stasher.u8(0),
        }
        stasher.object(&self.argument_scope);
    }
}

impl<'a> Unstashable<UnstashingContext<'a>> for BasicProcessorInput {
    fn unstash(unstasher: &mut Unstasher<UnstashingContext>) -> Result<Self, UnstashError> {
        let id = ProcessorInputId::new(unstasher.u64()? as _);

        let options = match unstasher.u8()? {
            0 => InputOptions::Synchronous,
            1 => InputOptions::NonSynchronous,
            _ => panic!(),
        };

        let branches = unstasher.u64()? as usize;

        let target = match unstasher.u8()? {
            1 => Some(SoundProcessorId::new(unstasher.u64()? as _)),
            0 => None,
            _ => panic!(),
        };

        let argument_scope = unstasher.object()?;

        Ok(BasicProcessorInput {
            id,
            options,
            branches,
            target,
            argument_scope,
        })
    }
}

impl<'a> UnstashableInplace<UnstashingContext<'a>> for BasicProcessorInput {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext>,
    ) -> Result<(), UnstashError> {
        // TODO: this code duplication could be avoided with an InplaceUnstasher
        // method that reuses a Unstashable implementation *without* inserting
        // an object value type

        let id = ProcessorInputId::new(unstasher.u64_always()? as _);

        let options = match unstasher.u8_always()? {
            0 => InputOptions::Synchronous,
            1 => InputOptions::NonSynchronous,
            _ => panic!(),
        };

        let branches = unstasher.u64_always()? as usize;

        let target = match unstasher.u8_always()? {
            1 => Some(SoundProcessorId::new(unstasher.u64_always()? as _)),
            0 => None,
            _ => panic!(),
        };

        let argument_scope = unstasher.object_always()?;

        if unstasher.time_to_write() {
            *self = BasicProcessorInput {
                id,
                options,
                branches,
                target,
                argument_scope,
            };
        }

        Ok(())
    }
}

pub struct InputContext<'a> {
    audio_context: &'a AudioContext<'a>,
    argument_stack: ArgumentStackView<'a>,
}

impl<'a> InputContext<'a> {
    pub fn new(audio_context: &'a AudioContext<'a>) -> InputContext<'a> {
        InputContext {
            audio_context,
            argument_stack: audio_context.argument_stack().clone(),
        }
    }

    pub fn push<T: ArgumentTranslation>(
        mut self,
        arg: CompiledProcessorArgument<T>,
        value: T::PushedType<'_>,
    ) -> InputContext<'a> {
        let converted = T::convert_value(value);
        self.argument_stack.push(arg.id(), converted);
        self
    }

    pub(crate) fn audio_context(&self) -> &AudioContext<'a> {
        self.audio_context
    }

    pub(crate) fn argument_stack(&self) -> ArgumentStackView<'a> {
        self.argument_stack
    }
}

pub trait AnyProcessorInput {
    fn id(&self) -> ProcessorInputId;
}
