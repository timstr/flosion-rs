use parking_lot::RwLock;

use crate::core::{
    context::ProcessorContext,
    graphobject::{ObjectType, WithObjectType},
    numeric,
    soundchunk::SoundChunk,
    soundinput::{InputOptions, SingleSoundInputHandle, SoundInputId},
    soundprocessor::DynamicSoundProcessor,
    soundprocessortools::SoundProcessorTools,
    soundstate::EmptyState,
};

pub struct Mixer {
    inputs: RwLock<Vec<SingleSoundInputHandle>>,
}

const MIXER_INPUT_OPTIONS: InputOptions = InputOptions {
    interruptible: false,
    realtime: true,
};

impl Mixer {
    pub fn add_input(&self, tools: &mut SoundProcessorTools<'_, EmptyState>) {
        self.inputs
            .write()
            .push(tools.add_single_sound_input(MIXER_INPUT_OPTIONS))
    }

    pub fn remove_input(&self, id: SoundInputId, tools: &mut SoundProcessorTools<'_, EmptyState>) {
        let mut inputs = self.inputs.write();
        debug_assert!(inputs.iter().filter(|h| h.id() == id).count() == 1);
        let i = inputs.iter().position(|h| h.id() == id).unwrap();
        let h = inputs.remove(i);
        tools.remove_single_sound_input(h);
    }

    pub fn get_input_ids(&self) -> Vec<SoundInputId> {
        self.inputs.read().iter().map(|h| h.id()).collect()
    }
}

impl DynamicSoundProcessor for Mixer {
    type StateType = EmptyState;

    fn new_default(tools: &mut SoundProcessorTools<'_, EmptyState>) -> Mixer {
        Mixer {
            inputs: RwLock::new(vec![
                tools.add_single_sound_input(MIXER_INPUT_OPTIONS),
                tools.add_single_sound_input(MIXER_INPUT_OPTIONS),
            ]),
        }
    }

    fn process_audio(&self, dst: &mut SoundChunk, mut context: ProcessorContext<'_, EmptyState>) {
        let inputs = self.inputs.read();
        if inputs.len() == 0 {
            dst.silence();
            return;
        }
        context.step_single_input(inputs.first().unwrap(), dst);
        let mut ch = SoundChunk::new();
        for i in &inputs[1..] {
            context.step_single_input(i, &mut ch);
            numeric::add_inplace(&mut dst.l, &ch.l);
            numeric::add_inplace(&mut dst.r, &ch.r);
        }
    }
}

impl WithObjectType for Mixer {
    const TYPE: ObjectType = ObjectType::new("mixer");
}
