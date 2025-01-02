use std::ops::{Deref, DerefMut};

use hashstash::{
    InplaceUnstasher, Stashable, Stasher, UnstashError, Unstashable, UnstashableInplace, Unstasher,
};

use crate::core::{
    engine::soundgraphcompiler::SoundGraphCompiler,
    jit::argumentstack::ArgumentStackView,
    soundchunk::CHUNK_SIZE,
    stashing::{StashingContext, UnstashingContext},
    uniqueid::UniqueId,
};

use super::{
    argument::{ArgumentScope, ArgumentTranslation, CompiledProcessorArgument},
    context::AudioContext,
    soundprocessor::{
        ProcessorComponent, ProcessorComponentVisitor, ProcessorComponentVisitorMut,
        SoundProcessorId, StartOver,
    },
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
pub enum SoundInputCategory {
    /// The input compiles to one node, which may be called upon
    /// any number of times per chunk and started over at will
    Anisochronic,

    /// The input compiles to one node which is called exactly once
    /// per chunk at all times and is only started over when the
    /// calling processor starts over. This is a requirement for
    /// calling on static processor nodes and for sharing cached
    /// audio chunks between nodes (which has not yet been implemented)
    Isochronic,

    /// The input compiles to a variable number of nodes. How those
    /// nodes are called and started over is of secondary imporance.
    Branched(usize),

    /// The input compiles to one node which follows a defined and
    /// inspectable schedule
    Scheduled,
}

impl SoundInputCategory {
    pub(crate) fn is_isochronic(&self) -> bool {
        match self {
            SoundInputCategory::Isochronic => true,
            _ => false,
        }
    }

    pub(crate) fn count_branches(&self) -> usize {
        match self {
            SoundInputCategory::Anisochronic => 1,
            SoundInputCategory::Isochronic => 1,
            SoundInputCategory::Branched(n) => *n,
            SoundInputCategory::Scheduled => 1,
        }
    }
}

impl Stashable<StashingContext> for SoundInputCategory {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        match self {
            SoundInputCategory::Anisochronic => stasher.u8(0),
            SoundInputCategory::Isochronic => stasher.u8(1),
            SoundInputCategory::Branched(n) => {
                stasher.u8(2);
                stasher.u64(*n as _);
            }
            SoundInputCategory::Scheduled => stasher.u8(3),
        }
    }
}

impl<'a> Unstashable<UnstashingContext<'a>> for SoundInputCategory {
    fn unstash(unstasher: &mut Unstasher<UnstashingContext<'a>>) -> Result<Self, UnstashError> {
        Ok(match unstasher.u8()? {
            0 => SoundInputCategory::Anisochronic,
            1 => SoundInputCategory::Isochronic,
            2 => SoundInputCategory::Branched(unstasher.u64()? as _),
            3 => SoundInputCategory::Scheduled,
            _ => panic!(),
        })
    }
}

// TODO: this state should probably only be stored for those inputs
// that actually care about releases. The audio stack should allow
// such events to be passed through multiple processors without individual
// processors needing to be aware if they don't need to care.
#[derive(Clone, Copy, Eq, PartialEq)]
enum ReleaseStatus {
    NotYet,
    Pending { offset: usize },
    Released,
}

// TODO: break up
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

pub trait SoundInputBackend {
    type CompiledType<'ctx>: Send + StartOver;

    fn category(&self) -> SoundInputCategory;

    fn compile<'ctx>(
        &self,
        location: SoundInputLocation,
        target: Option<SoundProcessorId>,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx>;
}

#[derive(Eq, PartialEq, Debug)]
pub struct ProcessorInput<T> {
    id: ProcessorInputId,
    target: Option<SoundProcessorId>,
    argument_scope: ArgumentScope,
    backend: T,
}

impl<T> ProcessorInput<T> {
    pub fn new_from_parts(argument_scope: ArgumentScope, backend: T) -> ProcessorInput<T> {
        ProcessorInput {
            id: ProcessorInputId::new_unique(),
            target: None,
            argument_scope,
            backend,
        }
    }
}

pub trait AnyProcessorInput {
    fn id(&self) -> ProcessorInputId;

    fn target(&self) -> Option<SoundProcessorId>;
    fn set_target(&mut self, target: Option<SoundProcessorId>);

    fn argument_scope(&self) -> &ArgumentScope;

    fn category(&self) -> SoundInputCategory;
}

impl<T: SoundInputBackend> AnyProcessorInput for ProcessorInput<T> {
    fn id(&self) -> ProcessorInputId {
        self.id
    }

    fn target(&self) -> Option<SoundProcessorId> {
        self.target
    }

    fn set_target(&mut self, target: Option<SoundProcessorId>) {
        self.target = target;
    }

    fn argument_scope(&self) -> &ArgumentScope {
        &self.argument_scope
    }

    fn category(&self) -> SoundInputCategory {
        self.backend.category()
    }
}

impl<T: SoundInputBackend> ProcessorComponent for ProcessorInput<T> {
    type CompiledType<'ctx> = T::CompiledType<'ctx>;

    fn visit<'a>(&self, visitor: &'a mut dyn ProcessorComponentVisitor) {
        visitor.input(self);
    }

    fn visit_mut<'a>(&mut self, visitor: &'a mut dyn ProcessorComponentVisitorMut) {
        visitor.input(self);
    }

    fn compile<'ctx>(
        &self,
        processor_id: SoundProcessorId,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx> {
        self.backend.compile(
            SoundInputLocation::new(processor_id, self.id),
            self.target,
            compiler,
        )
    }
}

impl<T> Deref for ProcessorInput<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.backend
    }
}

impl<T> DerefMut for ProcessorInput<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.backend
    }
}

impl<T: Stashable<StashingContext>> Stashable<StashingContext> for ProcessorInput<T> {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.u64(self.id.value() as _);
        match self.target {
            None => stasher.u8(0),
            Some(spid) => {
                stasher.u8(1);
                stasher.u64(spid.value() as _);
            }
        }
        stasher.object(&self.argument_scope);
        stasher.object(&self.backend);
    }
}

impl<'a, T: 'static + Unstashable<UnstashingContext<'a>>> Unstashable<UnstashingContext<'a>>
    for ProcessorInput<T>
{
    fn unstash(unstasher: &mut Unstasher<UnstashingContext<'a>>) -> Result<Self, UnstashError> {
        let id = ProcessorInputId::new(unstasher.u64()? as _);
        let target = match unstasher.u8()? {
            0 => None,
            1 => Some(SoundProcessorId::new(unstasher.u64()? as _)),
            _ => panic!(),
        };
        let argument_scope = unstasher.object()?;
        let backend = unstasher.object()?;
        Ok(ProcessorInput {
            id,
            target,
            argument_scope,
            backend,
        })
    }
}

impl<'a, T: 'static + UnstashableInplace<UnstashingContext<'a>>>
    UnstashableInplace<UnstashingContext<'a>> for ProcessorInput<T>
{
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext<'a>>,
    ) -> Result<(), UnstashError> {
        let id = ProcessorInputId::new(unstasher.u64_always()? as _);
        let target = match unstasher.u8_always()? {
            0 => None,
            1 => Some(SoundProcessorId::new(unstasher.u64_always()? as _)),
            _ => panic!(),
        };

        if unstasher.time_to_write() {
            self.id = id;
            self.target = target;
        }

        unstasher.object_inplace(&mut self.argument_scope)?;
        unstasher.object_inplace(&mut self.backend)?;

        Ok(())
    }
}
