use crate::{
    core::{
        engine::soundgraphcompiler::SoundGraphCompiler,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            context::{Context, LocalArrayList},
            soundinput::{InputOptions, SoundInputId},
            soundinputtypes::{SingleInputList, SingleInputListNode},
            soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
            soundprocessortools::SoundProcessorTools,
        },
        soundchunk::SoundChunk,
    },
    ui_core::arguments::{NaturalNumberArgument, ParsedArguments},
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

    pub const ARG_NUM_INPUTS: NaturalNumberArgument = NaturalNumberArgument("num_inputs");
}

impl DynamicSoundProcessor for Mixer {
    type StateType = ();
    type SoundInputType = SingleInputList;
    type Expressions<'ctx> = ();

    fn new(mut tools: SoundProcessorTools, args: &ParsedArguments) -> Result<Self, ()> {
        let num_inputs = args.get(&Mixer::ARG_NUM_INPUTS).unwrap_or(2);
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

    fn compile_expressions<'a, 'ctx>(
        &self,
        _compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        ()
    }

    fn process_audio(
        state: &mut StateAndTiming<()>,
        sound_inputs: &mut SingleInputListNode,
        _expressions: &mut (),
        dst: &mut SoundChunk,
        mut context: Context,
    ) -> StreamStatus {
        let mut ipts = sound_inputs.items_mut();
        let first_input = ipts.next();
        let mut first_input = match first_input {
            Some(i) => i,
            None => {
                dst.silence();
                return StreamStatus::Done;
            }
        };
        let mut all_done;
        {
            first_input.step(state, dst, &mut context, LocalArrayList::new());
            all_done = first_input.timing().is_done();
        }
        let mut ch = SoundChunk::new();
        for mut i in ipts {
            if i.timing().is_done() {
                continue;
            }
            all_done = false;
            i.step(state, &mut ch, &mut context, LocalArrayList::new());
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
