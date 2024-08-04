use std::hash::Hasher;

use hashrevise::{Revisable, RevisionHash, RevisionHasher};

use crate::core::sound::{
    soundgraphdata::SoundInputData,
    soundgraphtopology::SoundGraphTopology,
    soundinput::{InputOptions, SoundInputId},
    soundprocessor::SoundProcessorId,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct InterconnectInput {
    pub id: SoundInputId,
    pub options: InputOptions,
    pub branches: usize,
}

impl InterconnectInput {
    pub(crate) fn from_input_data(data: &SoundInputData) -> InterconnectInput {
        InterconnectInput {
            id: data.id(),
            options: data.options(),
            branches: data.branches().len(),
        }
    }
}

impl Revisable for InterconnectInput {
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = RevisionHasher::new();
        hasher.write_revisable(&self.id);
        hasher.write_u8(match self.options {
            InputOptions::Synchronous => 0,
            InputOptions::NonSynchronous => 1,
        });
        hasher.write_usize(self.branches);
        hasher.into_revision()
    }
}

/// Describes the spaces around and between sound processors in a stacked
/// group, in terms of which processors and which sound input meet at
/// the region of space.
// TODO: rename this
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessorInterconnect {
    /// The interconnect at the top of a stack and on above the top
    /// sound processor. Note that a processor with no inputs does
    /// not correspond to any interconnects, and similarly, a processor
    /// with multiple inputs will have many interconnects.
    TopOfStack(SoundProcessorId, InterconnectInput),

    /// The interconnect between two sound processors in the interior
    /// of a stacked group, where the bottom processor has exactly one
    /// sound input which is connected to the top sound processor, and
    /// by virtue of the stacked group, the top processor is not connected
    /// to any other inputs.
    BetweenTwoProcessors {
        bottom: SoundProcessorId,
        top: SoundProcessorId,
        input: InterconnectInput,
    },

    /// The space below the lowest sound processor in the stacked group.
    BottomOfStack(SoundProcessorId),
}

impl ProcessorInterconnect {
    pub(crate) fn processor_above(&self) -> Option<SoundProcessorId> {
        match self {
            ProcessorInterconnect::TopOfStack(_, _) => None,
            ProcessorInterconnect::BetweenTwoProcessors {
                bottom: _,
                top,
                input: _,
            } => Some(*top),
            ProcessorInterconnect::BottomOfStack(i) => Some(*i),
        }
    }

    pub(crate) fn processor_below(&self) -> Option<SoundProcessorId> {
        match self {
            ProcessorInterconnect::TopOfStack(i, _) => Some(*i),
            ProcessorInterconnect::BetweenTwoProcessors {
                bottom,
                top: _,
                input: _,
            } => Some(*bottom),
            ProcessorInterconnect::BottomOfStack(_) => None,
        }
    }

    pub(crate) fn unique_input(&self) -> Option<InterconnectInput> {
        match self {
            ProcessorInterconnect::TopOfStack(_, i) => Some(*i),
            ProcessorInterconnect::BetweenTwoProcessors {
                bottom: _,
                top: _,
                input,
            } => Some(*input),
            ProcessorInterconnect::BottomOfStack(_) => None,
        }
    }

    pub(crate) fn includes_processor(&self, processor: SoundProcessorId) -> bool {
        match self {
            ProcessorInterconnect::TopOfStack(spid, _) => processor == *spid,
            ProcessorInterconnect::BetweenTwoProcessors {
                bottom,
                top,
                input: _,
            } => [*bottom, *top].contains(&processor),
            ProcessorInterconnect::BottomOfStack(spid) => processor == *spid,
        }
    }

    pub(crate) fn is_below_processor(&self, processor: SoundProcessorId) -> bool {
        match self {
            ProcessorInterconnect::TopOfStack(_, _) => false,
            ProcessorInterconnect::BetweenTwoProcessors {
                bottom: _,
                top,
                input: _,
            } => *top == processor,
            ProcessorInterconnect::BottomOfStack(spid) => *spid == processor,
        }
    }

    /// Returns true iff the graph ids belonging to the interconnect
    /// all refer to objects that exist in the given topology
    pub(crate) fn is_valid(&self, topo: &SoundGraphTopology) -> bool {
        match self {
            ProcessorInterconnect::TopOfStack(spid, ii) => {
                topo.contains(spid) && topo.contains(ii.id)
            }
            ProcessorInterconnect::BetweenTwoProcessors { bottom, top, input } => {
                topo.contains(bottom) && topo.contains(top) && topo.contains(input.id)
            }
            ProcessorInterconnect::BottomOfStack(spid) => topo.contains(spid),
        }
    }
}

impl Revisable for ProcessorInterconnect {
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = RevisionHasher::new();
        match self {
            ProcessorInterconnect::TopOfStack(spid, input) => {
                hasher.write_u8(0);
                hasher.write_revisable(spid);
                hasher.write_revisable(input);
            }
            ProcessorInterconnect::BetweenTwoProcessors { bottom, top, input } => {
                hasher.write_u8(1);
                hasher.write_revisable(bottom);
                hasher.write_revisable(top);
                hasher.write_revisable(input);
            }
            ProcessorInterconnect::BottomOfStack(spid) => {
                hasher.write_u8(2);
                hasher.write_revisable(spid);
            }
        }
        hasher.into_revision()
    }
}
