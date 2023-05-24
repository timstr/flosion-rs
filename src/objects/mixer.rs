use crate::core::{
    context::Context,
    graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    numeric,
    serialization::Serializer,
    soundchunk::SoundChunk,
    soundinput::{InputOptions, SoundInputId},
    soundinputtypes::{SingleInputList, SingleInputListNode},
    soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
    soundprocessortools::SoundProcessorTools,
};

pub struct Mixer {
    inputs: SingleInputList,
}

const MIXER_INPUT_OPTIONS: InputOptions = InputOptions::Synchronous;

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

    pub fn num_inputs(&self) -> usize {
        self.inputs.length()
    }
}

impl DynamicSoundProcessor for Mixer {
    type StateType = ();
    type SoundInputType = SingleInputList;
    type NumberInputType<'ctx> = ();

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        let num_inputs: usize = match _init {
            ObjectInitialization::Args(_) => {
                // TODO: add argument
                2
            }
            ObjectInitialization::Archive(mut a) => a.u8()? as usize,
            ObjectInitialization::Default => 2,
        };
        Ok(Mixer {
            inputs: SingleInputList::new(num_inputs, MIXER_INPUT_OPTIONS, &mut tools),
        })
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &self.inputs
    }

    fn make_state(&self) -> Self::StateType {
        ()
    }

    fn make_number_inputs<'ctx>(
        &self,
        _context: &'ctx inkwell::context::Context,
    ) -> Self::NumberInputType<'ctx> {
        ()
    }

    fn process_audio(
        state: &mut StateAndTiming<()>,
        sound_inputs: &mut SingleInputListNode,
        _number_inputs: &(),
        dst: &mut SoundChunk,
        mut context: Context,
    ) -> StreamStatus {
        let ipts = sound_inputs.get_mut();
        if ipts.is_empty() {
            dst.silence();
            return StreamStatus::Done;
        }
        let mut all_done;
        {
            let first_input = ipts.first_mut().unwrap();
            if first_input.needs_reset() {
                first_input.reset(0);
            }
            first_input.step(state, dst, &mut context);
            all_done = first_input.is_done();
        }
        let mut ch = SoundChunk::new();
        for i in &mut ipts[1..] {
            if i.needs_reset() {
                i.reset(0);
            }
            if i.is_done() {
                continue;
            }
            all_done = false;
            i.step(state, &mut ch, &mut context);
            numeric::add_inplace(&mut dst.l, &ch.l);
            numeric::add_inplace(&mut dst.r, &ch.r);
        }
        if all_done {
            StreamStatus::Done
        } else {
            StreamStatus::Playing
        }
    }

    fn serialize(&self, mut serializer: Serializer) {
        serializer.u8(self.inputs.length() as u8);
    }
}

impl WithObjectType for Mixer {
    const TYPE: ObjectType = ObjectType::new("mixer");
}
