use hashstash::{
    InplaceUnstasher, Order, Stashable, Stasher, UnstashError, Unstashable, UnstashableInplace,
    Unstasher,
};

use crate::core::{
    engine::{compiledprocessor::CompiledSoundInputNode, soundgraphcompiler::SoundGraphCompiler},
    sound::{
        argument::ArgumentScope,
        soundinput::{
            InputContext, ProcessorInput, SoundInputBackend, SoundInputCategory, SoundInputLocation,
        },
        soundprocessor::{
            CompiledComponentVisitor, CompiledProcessorComponent, SoundProcessorId, StartOver,
            StreamStatus,
        },
    },
    soundchunk::{SoundChunk, CHUNK_SIZE},
    stashing::{StashingContext, UnstashingContext},
    uniqueid::UniqueId,
};

pub struct InputTimeSpanTag;

pub type InputTimeSpanId = UniqueId<InputTimeSpanTag>;

#[derive(Clone, Copy, Debug)]
pub struct InputTimeSpan {
    id: InputTimeSpanId,
    start_sample: usize,
    length_samples: usize,
}

impl InputTimeSpan {
    pub(crate) fn new(
        id: InputTimeSpanId,
        start_sample: usize,
        length_samples: usize,
    ) -> InputTimeSpan {
        InputTimeSpan {
            id,
            start_sample,
            length_samples,
        }
    }

    pub(crate) fn id(&self) -> InputTimeSpanId {
        self.id
    }

    pub(crate) fn start_sample(&self) -> usize {
        self.start_sample
    }

    pub(crate) fn length_samples(&self) -> usize {
        self.length_samples
    }

    pub(crate) fn set_start_sample(&mut self, start_sample: usize) {
        self.start_sample = start_sample;
    }

    pub(crate) fn intersects_with(&self, other: InputTimeSpan) -> bool {
        let self_end = self.start_sample + self.length_samples;
        let other_end = other.start_sample + other.length_samples;

        !(other_end < self.start_sample || other.start_sample >= self_end)
    }
}

#[derive(Clone)]
pub struct SoundInputSchedule {
    spans: Vec<InputTimeSpan>,
}

impl SoundInputSchedule {
    pub fn new() -> SoundInputSchedule {
        SoundInputSchedule { spans: Vec::new() }
    }

    pub fn spans(&self) -> &[InputTimeSpan] {
        &self.spans
    }

    pub fn replace_spans(&mut self, mut spans: Vec<InputTimeSpan>) -> Result<(), ()> {
        if spans.is_empty() {
            self.spans.clear();
            return Ok(());
        }

        spans.sort_by_key(|s| s.start_sample);

        if spans.iter().zip(&spans[1..]).any(|(lspan, rspan)| {
            (lspan.start_sample() + lspan.length_samples()) > rspan.start_sample()
        }) {
            return Err(());
        };

        self.spans = spans;

        Ok(())
    }

    pub fn add_span(&mut self, start_sample: usize, length_samples: usize) -> Result<(), ()> {
        let span = InputTimeSpan::new(InputTimeSpanId::new_unique(), start_sample, length_samples);

        if self.spans.iter().any(|s| s.intersects_with(span)) {
            return Err(());
        }

        self.spans.push(span);
        self.spans.sort_by_key(|s| s.start_sample);

        Ok(())
    }
}

impl Stashable for SoundInputSchedule {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.array_of_proxy_objects(
            self.spans.iter(),
            |span, stasher| {
                stasher.object(&span.id);
                stasher.u64(span.start_sample as _);
                stasher.u64(span.length_samples as _);
            },
            Order::Unordered,
        );
    }
}

impl Unstashable for SoundInputSchedule {
    fn unstash(unstasher: &mut Unstasher) -> Result<SoundInputSchedule, UnstashError> {
        let mut spans = Vec::new();

        unstasher.array_of_proxy_objects(|unstasher| {
            spans.push(InputTimeSpan {
                id: unstasher.object()?,
                start_sample: unstasher.u64()? as _,
                length_samples: unstasher.u64()? as _,
            });
            Ok(())
        })?;

        Ok(SoundInputSchedule { spans })
    }
}

impl UnstashableInplace for SoundInputSchedule {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        let time_to_write = unstasher.time_to_write();

        if time_to_write {
            self.spans.clear();
        }

        unstasher.array_of_proxy_objects(|unstasher| {
            let span = InputTimeSpan {
                id: unstasher.object()?,
                start_sample: unstasher.u64()? as _,
                length_samples: unstasher.u64()? as _,
            };
            if time_to_write {
                self.spans.push(span);
            }
            Ok(())
        })?;

        Ok(())
    }
}

pub struct ScheduledInputBackend {
    schedule: SoundInputSchedule,
}

impl ScheduledInputBackend {
    pub fn schedule_mut(&mut self) -> &mut SoundInputSchedule {
        &mut self.schedule
    }
}

impl SoundInputBackend for ScheduledInputBackend {
    type CompiledType<'ctx> = CompiledScheduledInput<'ctx>;

    fn category(&self) -> SoundInputCategory {
        SoundInputCategory::Scheduled
    }

    fn schedule(&self) -> Option<&SoundInputSchedule> {
        Some(&self.schedule)
    }

    fn schedule_mut(&mut self) -> Option<&mut SoundInputSchedule> {
        Some(&mut self.schedule)
    }

    fn compile<'ctx>(
        &self,
        location: SoundInputLocation,
        target: Option<SoundProcessorId>,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx> {
        CompiledScheduledInput {
            node: CompiledSoundInputNode::new(location, compiler.compile_sound_processor(target)),
            schedule: self.schedule.clone(),
            scratch_buffer: SoundChunk::new(),
            scratch_offset: 0,
        }
    }
}

impl Stashable<StashingContext> for ScheduledInputBackend {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object_with_context(&self.schedule, ());
    }
}

impl Unstashable<UnstashingContext<'_>> for ScheduledInputBackend {
    fn unstash(
        unstasher: &mut Unstasher<UnstashingContext>,
    ) -> Result<ScheduledInputBackend, UnstashError> {
        Ok(ScheduledInputBackend {
            schedule: unstasher.object_with_context(())?,
        })
    }
}

impl UnstashableInplace<UnstashingContext<'_>> for ScheduledInputBackend {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext>,
    ) -> Result<(), UnstashError> {
        unstasher.object_inplace_with_context(&mut self.schedule, ())?;
        Ok(())
    }
}

pub struct CompiledScheduledInput<'ctx> {
    node: CompiledSoundInputNode<'ctx>,
    schedule: SoundInputSchedule,
    scratch_buffer: SoundChunk,
    scratch_offset: usize,
}

impl<'ctx> CompiledScheduledInput<'ctx> {
    pub fn step(&mut self, dst: &mut SoundChunk, context: InputContext) -> StreamStatus {
        let dst_begin = context
            .audio_context()
            .current_processor_timing()
            .elapsed_chunks()
            * CHUNK_SIZE;
        let dst_end = dst_begin + CHUNK_SIZE;

        dst.silence();

        for span in self.schedule.spans() {
            let span_end = span.start_sample + span.length_samples;

            // If the span doesn't overlap with the chunk, ignore it
            if span.start_sample >= dst_end || span_end < dst_begin {
                continue;
            }

            if span.start_sample > dst_begin && span.start_sample < dst_end {
                // If the span starts this chunk, restart the input node
                let offset = span.start_sample - dst_begin;
                // TODO: use offset when starting over input, but first make this
                // interface a bit clearer
                self.node.start_over_at(0);
                self.scratch_offset = offset;
            } else {
                // If the span started before, copy the remainder of the scratch buffer
                // to the output

                let scratch_split = CHUNK_SIZE - self.scratch_offset;
                let early_end = (dst_begin + self.scratch_offset).saturating_sub(span_end);
                let end_of_dst = self.scratch_offset - early_end;
                let end_of_scratch_buffer = CHUNK_SIZE - early_end;
                dst.l[..end_of_dst]
                    .copy_from_slice(&self.scratch_buffer.l[scratch_split..end_of_scratch_buffer]);
                dst.r[..end_of_dst]
                    .copy_from_slice(&self.scratch_buffer.r[scratch_split..end_of_scratch_buffer]);
                if early_end > 0 {
                    continue;
                }
            }

            self.node.step(&mut self.scratch_buffer, context.clone());

            // Copy from the front of the scratch buffer to the output
            let scratch_split = CHUNK_SIZE - self.scratch_offset;
            let early_end = dst_end.saturating_sub(span_end);
            let end_of_dst = CHUNK_SIZE - early_end;
            let end_of_scratch_buffer = scratch_split - early_end;
            dst.l[self.scratch_offset..end_of_dst]
                .copy_from_slice(&self.scratch_buffer.l[..end_of_scratch_buffer]);
            dst.r[self.scratch_offset..end_of_dst]
                .copy_from_slice(&self.scratch_buffer.r[..end_of_scratch_buffer]);
        }

        StreamStatus::Playing
    }
}

impl<'ctx> CompiledProcessorComponent for CompiledScheduledInput<'ctx> {
    fn visit(&self, visitor: &mut dyn CompiledComponentVisitor) {
        visitor.input_node(&self.node);
    }
}

impl<'ctx> StartOver for CompiledScheduledInput<'ctx> {
    fn start_over(&mut self) {
        self.node.start_over_at(0);
    }
}

pub type ScheduledInput = ProcessorInput<ScheduledInputBackend>;

impl ScheduledInput {
    pub fn new(argument_scope: ArgumentScope) -> ScheduledInput {
        ProcessorInput::new_from_parts(
            argument_scope,
            ScheduledInputBackend {
                schedule: SoundInputSchedule::new(),
            },
        )
    }
}
