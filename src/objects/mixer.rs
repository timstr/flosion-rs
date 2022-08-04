use crate::core::{
    context::Context,
    graphobject::{ObjectType, WithObjectType},
    numeric,
    soundchunk::SoundChunk,
    soundinput::{InputOptions, SoundInputId},
    soundprocessor::SoundProcessor,
    soundprocessortools::SoundProcessorTools,
    statetree::{NoState, SingleInputList, SingleInputListNode},
};

pub struct Mixer {
    inputs: SingleInputList,
}

const MIXER_INPUT_OPTIONS: InputOptions = InputOptions {
    interruptible: false,
    realtime: true,
};

impl Mixer {
    pub fn add_input(&self, tools: &mut SoundProcessorTools) {
        self.inputs.add_input(tools);
    }

    pub fn remove_input(&self, id: SoundInputId, tools: &mut SoundProcessorTools) {
        self.inputs.remove_input(id, tools);
    }

    pub fn get_input_ids(&self) -> Vec<SoundInputId> {
        self.inputs.get_input_ids()
    }
}

impl SoundProcessor for Mixer {
    const IS_STATIC: bool = false;

    type StateType = NoState;
    type InputType = SingleInputList;

    fn new(mut tools: SoundProcessorTools) -> Self {
        Mixer {
            inputs: SingleInputList::new(2, MIXER_INPUT_OPTIONS, &mut tools),
        }
    }

    fn get_input(&self) -> &Self::InputType {
        &self.inputs
    }

    fn make_state(&self) -> Self::StateType {
        NoState {}
    }

    fn process_audio(
        state: &mut NoState,
        inputs: &mut SingleInputListNode,
        dst: &mut SoundChunk,
        mut context: Context,
    ) {
        let ipts = inputs.get_mut();
        if ipts.len() == 0 {
            dst.silence();
            return;
        }
        ipts.first_mut().unwrap().step(state, dst, &mut context);
        let mut ch = SoundChunk::new();
        for i in &mut ipts[1..] {
            i.step(state, &mut ch, &mut context);
            numeric::add_inplace(&mut dst.l, &ch.l);
            numeric::add_inplace(&mut dst.r, &ch.r);
        }
    }
}

impl WithObjectType for Mixer {
    const TYPE: ObjectType = ObjectType::new("mixer");
}
