use flosion_macros::ProcessorComponents;
use hashstash::{InplaceUnstasher, Order, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::{
    core::{
        objecttype::{ObjectType, WithObjectType},
        sound::{
            argument::ArgumentScope,
            context::AudioContext,
            inputtypes::singleinput::SingleInput,
            soundinput::{Chronicity, InputContext, ProcessorInputId},
            soundprocessor::{SoundProcessor, StreamStatus},
        },
        soundchunk::SoundChunk,
        stashing::{StashingContext, UnstashingContext},
    },
    ui_core::arguments::{NaturalNumberArgument, ParsedArguments},
};

#[derive(ProcessorComponents)]
pub struct Mixer {
    inputs: Vec<SingleInput>,
}

const MIXER_INPUT_CHRONICITY: Chronicity = Chronicity::Iso;

impl Mixer {
    pub fn add_input(&mut self) {
        self.inputs.push(SingleInput::new(
            MIXER_INPUT_CHRONICITY,
            ArgumentScope::new_empty(),
        ));
    }

    pub fn remove_input(&mut self, id: ProcessorInputId) {
        self.inputs.retain(|i| i.id() != id);
    }

    pub fn inputs(&self) -> &[SingleInput] {
        &self.inputs
    }

    pub const ARG_NUM_INPUTS: NaturalNumberArgument = NaturalNumberArgument("num_inputs");
}

impl SoundProcessor for Mixer {
    fn new(args: &ParsedArguments) -> Mixer {
        let num_inputs = args.get(&Mixer::ARG_NUM_INPUTS).unwrap_or(2);
        Mixer {
            inputs: (0..num_inputs)
                .map(|_| SingleInput::new(MIXER_INPUT_CHRONICITY, ArgumentScope::new_empty()))
                .collect(),
        }
    }

    fn is_static(&self) -> bool {
        false
    }

    fn process_audio(
        mixer: &mut Self::CompiledType<'_>,
        dst: &mut SoundChunk,
        context: &mut AudioContext,
    ) -> StreamStatus {
        let mut inputs = mixer.inputs.iter_mut();
        let first_input = inputs.next();
        let first_input = match first_input {
            Some(i) => i,
            None => {
                dst.silence();
                return StreamStatus::Done;
            }
        };
        let mut all_done;
        {
            first_input.step(dst, InputContext::new(context));
            all_done = first_input.timing().is_done();
        }
        let mut ch = SoundChunk::new();
        for i in inputs {
            if i.timing().is_done() {
                continue;
            }
            all_done = false;
            i.step(&mut ch, InputContext::new(context));
            slicemath::add_inplace(&mut dst.l, &ch.l);
            slicemath::add_inplace(&mut dst.r, &ch.r);
        }
        if all_done {
            StreamStatus::Done
        } else {
            StreamStatus::Playing
        }
    }
}

impl WithObjectType for Mixer {
    const TYPE: ObjectType = ObjectType::new("mixer");
}

impl Stashable<StashingContext> for Mixer {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.array_of_objects_slice(&self.inputs, Order::Ordered);
    }
}

impl UnstashableInplace<UnstashingContext<'_>> for Mixer {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext<'_>>,
    ) -> Result<(), UnstashError> {
        unstasher.array_of_objects_vec_inplace(&mut self.inputs)
    }
}
