use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::{
    core::{
        engine::{compiledexpression::CompiledExpression, soundgraphcompiler::SoundGraphCompiler},
        expression::context::ExpressionContext,
        jit::compiledexpression::Discretization,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            context::Context,
            expression::{ProcessorExpression, SoundExpressionScope},
            soundprocessor::{
                CompiledSoundProcessor, ProcessorComponent, ProcessorComponentVisitor,
                ProcessorComponentVisitorMut, SoundProcessor, SoundProcessorId, StartOver,
                StreamStatus,
            },
        },
        soundchunk::SoundChunk,
    },
    ui_core::arguments::ParsedArguments,
};

pub struct WriteWaveform {
    pub waveform: ProcessorExpression,
}

pub struct CompiledWriteWaveform<'ctx> {
    waveform: CompiledExpression<'ctx>,
}

impl SoundProcessor for WriteWaveform {
    fn new(_args: &ParsedArguments) -> WriteWaveform {
        WriteWaveform {
            waveform: ProcessorExpression::new(0.0, SoundExpressionScope::with_processor_state()),
        }
    }

    fn is_static(&self) -> bool {
        false
    }
}

impl ProcessorComponent for WriteWaveform {
    type CompiledType<'ctx> = CompiledWriteWaveform<'ctx>;

    fn visit<'a>(&self, visitor: &'a mut dyn ProcessorComponentVisitor) {
        self.waveform.visit(visitor);
    }

    fn visit_mut<'a>(&mut self, visitor: &'a mut dyn ProcessorComponentVisitorMut) {
        self.waveform.visit_mut(visitor);
    }

    fn compile<'ctx>(
        &self,
        processor_id: SoundProcessorId,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx> {
        CompiledWriteWaveform {
            waveform: self.waveform.compile(processor_id, compiler),
        }
    }
}

impl<'ctx> StartOver for CompiledWriteWaveform<'ctx> {
    fn start_over(&mut self) {
        self.waveform.start_over();
    }
}

impl<'ctx> CompiledSoundProcessor<'ctx> for CompiledWriteWaveform<'ctx> {
    fn process_audio(&mut self, dst: &mut SoundChunk, context: &mut Context) -> StreamStatus {
        self.waveform.eval(
            &mut dst.l,
            Discretization::samplewise_temporal(),
            ExpressionContext::new_minimal(context),
        );
        slicemath::copy(&dst.l, &mut dst.r);

        StreamStatus::Playing
    }
}

impl WithObjectType for WriteWaveform {
    const TYPE: ObjectType = ObjectType::new("writewaveform");
}

impl Stashable for WriteWaveform {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.object(&self.waveform);
    }
}

impl UnstashableInplace for WriteWaveform {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.waveform)
    }
}
