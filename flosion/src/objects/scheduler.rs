use flosion_macros::ProcessorComponent;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::{
    core::{
        objecttype::{ObjectType, WithObjectType},
        samplefrequency::SAMPLE_FREQUENCY,
        sound::{
            argument::ArgumentScope,
            context::AudioContext,
            inputtypes::scheduledinput::ScheduledInput,
            soundinput::InputContext,
            soundprocessor::{SoundProcessor, StreamStatus},
        },
        soundchunk::SoundChunk,
        stashing::{StashingContext, UnstashingContext},
    },
    ui_core::arguments::ParsedArguments,
};

#[derive(ProcessorComponent)]
pub struct Scheduler {
    pub sound_input: ScheduledInput,
}

impl SoundProcessor for Scheduler {
    fn new(_args: &ParsedArguments) -> Scheduler {
        let mut sound_input = ScheduledInput::new(ArgumentScope::new_empty());

        let quarter_second = SAMPLE_FREQUENCY / 4;
        // TESTING
        sound_input
            .schedule_mut()
            .add_span(1 * quarter_second, quarter_second)
            .unwrap();
        sound_input
            .schedule_mut()
            .add_span(3 * quarter_second, quarter_second)
            .unwrap();
        sound_input
            .schedule_mut()
            .add_span(5 * quarter_second, quarter_second)
            .unwrap();
        sound_input
            .schedule_mut()
            .add_span(7 * quarter_second, quarter_second)
            .unwrap();

        Scheduler { sound_input }
    }

    fn is_static(&self) -> bool {
        false
    }

    fn process_audio(
        scheduler: &mut Self::CompiledType<'_>,
        dst: &mut SoundChunk,
        context: &mut AudioContext,
    ) -> StreamStatus {
        scheduler.sound_input.step(dst, InputContext::new(&context))
    }
}

impl WithObjectType for Scheduler {
    const TYPE: ObjectType = ObjectType::new("scheduler");
}

impl Stashable<StashingContext> for Scheduler {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object(&self.sound_input);
    }
}

impl<'a> UnstashableInplace<UnstashingContext<'a>> for Scheduler {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext<'a>>,
    ) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.sound_input)?;
        Ok(())
    }
}
