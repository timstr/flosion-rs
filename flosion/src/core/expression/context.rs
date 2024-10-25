use crate::core::{
    jit::argumentstack::ArgumentStackView,
    sound::{
        context::Context,
        argument::{ArgumentTranslation, CompiledProcessorArgument},
        soundinput::SoundInputLocation,
        soundprocessor::SoundProcessorId,
    },
};

pub struct ExpressionContext<'a> {
    audio_context: &'a Context<'a>,
    argument_stack: ArgumentStackView<'a>,
}

impl<'a> ExpressionContext<'a> {
    pub fn new(audio_context: &'a Context<'a>) -> ExpressionContext<'a> {
        ExpressionContext {
            audio_context,
            argument_stack: audio_context.argument_stack().clone(),
        }
    }

    pub fn push<T: ArgumentTranslation>(
        mut self,
        arg: CompiledProcessorArgument<T>,
        value: T::PushedType<'_>,
    ) -> ExpressionContext<'a> {
        let converted = T::convert_value(value);
        self.argument_stack.push(arg.id(), converted);
        self
    }

    pub(crate) fn audio_context(&self) -> &Context<'a> {
        self.audio_context
    }

    pub(crate) fn argument_stack(&self) -> &ArgumentStackView<'a> {
        &self.argument_stack
    }

    pub(crate) fn get_time_and_speed_at_sound_input(
        &self,
        location: SoundInputLocation,
    ) -> (f32, f32) {
        self.audio_context.time_offset_and_speed_at_input(location)
    }

    pub(crate) fn get_time_and_speed_at_sound_processor(&self, id: SoundProcessorId) -> (f32, f32) {
        self.audio_context.time_offset_and_speed_at_processor(id)
    }
}
