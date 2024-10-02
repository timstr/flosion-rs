use crate::core::sound::expressionargument::SoundExpressionArgumentOwner;

use super::{
    expression::ProcessorExpressionLocation, expressionargument::SoundExpressionArgumentId,
    path::SoundPath, soundgraph::SoundGraph, soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
};

#[derive(Debug, Eq, PartialEq)]
pub enum SoundError {
    ProcessorIdTaken(SoundProcessorId),
    ProcessorNotFound(SoundProcessorId),
    BadProcessorInit(SoundProcessorId),
    BadProcessorCleanup(SoundProcessorId),
    SoundInputIdTaken(SoundInputId),
    SoundInputNotFound(SoundInputId),
    BadSoundInputInit(SoundInputId),
    BadSoundInputCleanup(SoundInputId),
    BadSoundInputBranchIndex(SoundInputId, usize),
    SoundInputOccupied {
        input_id: SoundInputId,
        current_target: SoundProcessorId,
    },
    SoundInputUnoccupied(SoundInputId),
    CircularDependency {
        cycle: SoundPath,
    },
    StaticNotOneState(SoundProcessorId),
    StaticNotSynchronous(SoundProcessorId),
    ArgumentIdTaken(SoundExpressionArgumentId),
    ArgumentNotFound(SoundExpressionArgumentId),
    BadArgumentInit(SoundExpressionArgumentId),
    BadArgumentCleanup(SoundExpressionArgumentId),
    StateNotInScope {
        bad_dependencies: Vec<(SoundExpressionArgumentId, ProcessorExpressionLocation)>,
    },
}

impl SoundError {
    pub(crate) fn explain(&self, graph: &SoundGraph) -> String {
        match self {
            SoundError::ProcessorIdTaken(spid) => format!(
                "Processor id #{} is already taken by {}",
                spid.value(),
                graph.sound_processor(*spid).unwrap().friendly_name()
            ),
            SoundError::ProcessorNotFound(spid) => {
                format!("A processor with id #{} could not be found", spid.value())
            }
            SoundError::BadProcessorInit(spid) => format!(
                "The processor with id #{} was not initialized correctly",
                spid.value()
            ),
            SoundError::BadProcessorCleanup(spid) => format!(
                "The processor {} was not cleaned up correctly",
                graph.sound_processor(*spid).unwrap().friendly_name()
            ),
            SoundError::SoundInputIdTaken(siid) => {
                format!("Sound input id #{} is already taken", siid.value())
            }
            SoundError::SoundInputNotFound(siid) => {
                format!("A sound input with id #{} could not be found", siid.value())
            }
            SoundError::BadSoundInputInit(siid) => format!(
                "The sound input with id #{} was not initialized correctly",
                siid.value()
            ),
            SoundError::BadSoundInputCleanup(siid) => {
                let owner_spid = graph.sound_input(*siid).unwrap().owner();
                format!(
                    "The sound input with id #{} on processor {} was not cleaned up correctly",
                    siid.value(),
                    graph.sound_processor(owner_spid).unwrap().friendly_name()
                )
            }
            SoundError::BadSoundInputBranchIndex(siid, idx) => {
                let owner_spid = graph.sound_input(*siid).unwrap().owner();
                format!(
                    "The branch index {} is out of range for the sound input with id #{} of \
                    processor {}",
                    idx,
                    siid.value(),
                    graph.sound_processor(owner_spid).unwrap().friendly_name()
                )
            }
            SoundError::SoundInputOccupied {
                input_id,
                current_target,
            } => {
                let owner_spid = graph.sound_input(*input_id).unwrap().owner();
                format!(
                    "The sound input with id #{} of processor {} is already occupied and \
                    connected to {}",
                    input_id.value(),
                    graph.sound_processor(owner_spid).unwrap().friendly_name(),
                    graph
                        .sound_processor(*current_target)
                        .unwrap()
                        .friendly_name()
                )
            }
            SoundError::SoundInputUnoccupied(siid) => {
                let owner_spid = graph.sound_input(*siid).unwrap().owner();
                format!(
                    "The sound input with id #{} of processor {} is already unoccupied and not \
                    connected to anything",
                    siid.value(),
                    graph.sound_processor(owner_spid).unwrap().friendly_name(),
                )
            }
            SoundError::CircularDependency { cycle } => {
                let mut s = "The graph contains a cycle: ".to_string();

                let mut first = true;
                for (spid, siid) in &cycle.connections {
                    if !first {
                        s += " -> ";
                    }
                    s += &graph.sound_processor(*spid).unwrap().friendly_name();
                    s += &format!(" -(input #{})->", siid.value());
                    first = false;
                }
                s
            }
            SoundError::StaticNotOneState(spid) => format!(
                "The static processor {} needs to have exactly one state, but it \
                is connected to a branched sound input",
                graph.sound_processor(*spid).unwrap().friendly_name()
            ),
            SoundError::StaticNotSynchronous(spid) => format!(
                "The static processor {} is connected to a non-synchronous input",
                graph.sound_processor(*spid).unwrap().friendly_name()
            ),
            SoundError::ArgumentIdTaken(aid) => {
                format!("Argument id #{} is already taken", aid.value())
            }
            SoundError::ArgumentNotFound(aid) => {
                format!("An argument with id #{} could not be found", aid.value())
            }
            SoundError::BadArgumentInit(aid) => format!(
                "The argument with id #{} was not initialized correctly",
                aid.value()
            ),
            SoundError::BadArgumentCleanup(aid) => {
                let owner_str = match graph.expression_argument(*aid).unwrap().owner() {
                    SoundExpressionArgumentOwner::SoundProcessor(spid) => {
                        graph.sound_processor(spid).unwrap().friendly_name()
                    }
                    SoundExpressionArgumentOwner::SoundInput(siid) => {
                        let owner_spid = graph.sound_input(siid).unwrap().owner();
                        format!(
                            "sound input #{} of processor {}",
                            siid.value(),
                            graph.sound_processor(owner_spid).unwrap().friendly_name()
                        )
                    }
                };

                format!(
                    "The argument with id #{} of {} was not cleaned up correctly",
                    aid.value(),
                    owner_str
                )
            }
            SoundError::StateNotInScope {
                bad_dependencies: _,
            } => {
                format!(
                    "One or more expressions depend on arguments whose state is not available \
                    during evaluation because there isn't a unique sound path between the two. \
                    To be honest though, I don't think this should be a hard error, and it \
                    could be worked around creatively"
                )
            }
        }
    }
}
