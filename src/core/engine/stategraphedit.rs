use crate::core::{
    jit::compilednumberinput::CompiledNumberInputNode,
    number::numberinput::NumberInputId,
    sound::{soundinput::SoundInputId, soundprocessor::SoundProcessorId},
};

use super::stategraphnode::{NodeTargetValue, StateGraphNode};

// Edits to be made to the state graph on the audio thread.
// While StateGraphEdit is superficially similar to SoundEdit, it is heavily
// focused on efficiently inserted pre-allocated state graph data, rather
// than keeping track of the overall topology. Many edits made at the sound
// graph level have no analog or may not correspond to any edits here if
// they don't imply any individual changes to the state graph.
pub(crate) enum StateGraphEdit<'ctx> {
    AddStaticSoundProcessor(Box<dyn StateGraphNode<'ctx>>),
    RemoveStaticSoundProcessor(SoundProcessorId),
    AddSoundInput {
        input_id: SoundInputId,
        owner: SoundProcessorId,
        targets: Vec<NodeTargetValue<'ctx>>,
    },
    RemoveSoundInput {
        input_id: SoundInputId,
        owner_id: SoundProcessorId,
    },

    // TODO: does these make sense or should UpdateSoundInput achieve all of
    // this in one pass? Adding a sound input previously implied allocating
    // new node target values on the audio thread which is a no-no.
    // Lots of this is baked into the number input related interfaces which
    // could maybe benefit from being revisited, this time with the intention
    // of pre-allocating all data before it is given to the audio thread via
    // a StateGraphEdit here.
    AddSoundInputKey {
        input_id: SoundInputId,
        owner_id: SoundProcessorId,
        key_index: usize,
        targets: Vec<NodeTargetValue<'ctx>>,
    },
    RemoveSoundInputKey {
        input_id: SoundInputId,
        owner_id: SoundProcessorId,
        key_index: usize,
    },
    ReplaceSoundInputTargets {
        input_id: SoundInputId,
        owner_id: SoundProcessorId,
        targets: Vec<NodeTargetValue<'ctx>>,
    },
    UpdateNumberInput(NumberInputId, CompiledNumberInputNode<'ctx>),
}
