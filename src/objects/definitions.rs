use crate::core::{
    engine::{
        nodegen::NodeGen,
        soundnumberinputnode::{
            SoundNumberInputNode, SoundNumberInputNodeCollection, SoundNumberInputNodeVisitor,
            SoundNumberInputNodeVisitorMut,
        },
    },
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    jit::compilednumberinput::Discretization,
    sound::{
        context::{Context, LocalArrayList},
        soundinput::InputOptions,
        soundinputtypes::{SingleInput, SingleInputNode},
        soundnumberinput::SoundNumberInputHandle,
        soundnumbersource::{SoundNumberSourceHandle, SoundNumberSourceId},
        soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
        soundprocessortools::SoundProcessorTools,
    },
    soundchunk::{SoundChunk, CHUNK_SIZE},
};

pub struct Definitions {
    pub sound_input: SingleInput,

    // TODO: store these in a vector. Might need to rethink how DefinitionsNumberInputs works,
    // e.g. does it need to use Vec or can it use something friendlier to the audio thread?
    pub number_input: SoundNumberInputHandle,
    pub number_source: SoundNumberSourceHandle,
}

pub struct DefinitionsNumberInputs<'ctx> {
    input: SoundNumberInputNode<'ctx>,
    source_id: SoundNumberSourceId,
}

impl<'ctx> SoundNumberInputNodeCollection<'ctx> for DefinitionsNumberInputs<'ctx> {
    fn visit_number_inputs(&self, visitor: &mut dyn SoundNumberInputNodeVisitor<'ctx>) {
        visitor.visit_node(&self.input);
    }

    fn visit_number_inputs_mut(
        &mut self,
        visitor: &'_ mut dyn SoundNumberInputNodeVisitorMut<'ctx>,
    ) {
        visitor.visit_node(&mut self.input);
    }
}

impl DynamicSoundProcessor for Definitions {
    type StateType = ();

    type SoundInputType = SingleInput;

    type NumberInputType<'ctx> = DefinitionsNumberInputs<'ctx>;

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(Definitions {
            sound_input: SingleInput::new(InputOptions::Synchronous, &mut tools),
            number_input: tools.add_number_input(0.0),
            number_source: tools.add_local_array_number_source(),
        })
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &self.sound_input
    }

    fn make_state(&self) -> Self::StateType {
        ()
    }

    fn make_number_inputs<'a, 'ctx>(
        &self,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self::NumberInputType<'ctx> {
        DefinitionsNumberInputs {
            input: self.number_input.make_node(nodegen),
            source_id: self.number_source.id(),
        }
    }

    fn process_audio<'ctx>(
        state: &mut StateAndTiming<()>,
        sound_inputs: &mut SingleInputNode<'ctx>,
        number_inputs: &mut DefinitionsNumberInputs<'ctx>,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus {
        let mut buffer = context.get_scratch_space(CHUNK_SIZE);

        // TODO: fine-grained scoping rules for inside of sound processors.
        // Currently, the UI is prompting me to add a connection to the
        // definition's time number source, which panics when evaluated
        // if I haven't pushed the processor state onto the context.
        // In other cases, it isn't possible to do this, e.g. because
        // the processor state is being mutated by the number input itself.
        // Another problem is that it's currently legal to add a connection
        // here to the very buffer being evaluated, which will similarly
        // never be pushed to the context and thus will always panic.
        // What's a good way to represent how different number inputs have
        // access to different subsets of the processor's data in a way
        // that is faithful to common practices? This can and should be
        // enforced at the SoundGraph level so that attempting to make
        // a connection which is described as being locally out of scope
        // will produce an Err result.
        number_inputs.input.eval(
            &mut buffer,
            Discretization::samplewise_temporal(),
            &context.push_processor_state(state, LocalArrayList::new()),
        );

        // TODO: I don't like having to spell this out every time. It should
        // be automated while ensuring that fine-grained delays are still
        // possible
        if sound_inputs.timing().needs_reset() {
            sound_inputs.reset(0)
        }

        sound_inputs.step(
            state,
            dst,
            &context,
            LocalArrayList::new().push(&buffer, number_inputs.source_id),
        )
    }
}

impl WithObjectType for Definitions {
    const TYPE: ObjectType = ObjectType::new("definitions");
}
