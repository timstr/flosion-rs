use super::{
    expression::ProcessorExpressionLocation, expressionargument::ArgumentLocation,
    soundgraph::SoundGraph, soundinput::SoundInputId, soundprocessor::SoundProcessorId,
};

#[derive(Debug, Eq, PartialEq)]
pub enum SoundError {
    ProcessorNotFound(SoundProcessorId),
    SoundInputNotFound(SoundInputId),
    SoundInputOccupied {
        input_id: SoundInputId,
        current_target: SoundProcessorId,
    },
    SoundInputUnoccupied(SoundInputId),
    CircularDependency,
    StaticNotOneState(SoundProcessorId),
    StaticNotSynchronous(SoundProcessorId),
    StateNotInScope {
        bad_dependencies: Vec<(ArgumentLocation, ProcessorExpressionLocation)>,
    },
}

impl SoundError {
    pub(crate) fn explain(&self, graph: &SoundGraph) -> String {
        match self {
            SoundError::ProcessorNotFound(spid) => {
                format!("A processor with id #{} could not be found", spid.value())
            }
            SoundError::SoundInputNotFound(siid) => {
                format!("A sound input with id #{} could not be found", siid.value())
            }
            SoundError::SoundInputOccupied {
                input_id,
                current_target,
            } => {
                todo!()
            }
            SoundError::SoundInputUnoccupied(siid) => {
                todo!()
            }
            SoundError::CircularDependency => "The graph contains a cycle ".to_string(),
            SoundError::StaticNotOneState(spid) => format!(
                "The static processor {} needs to have exactly one state, but it \
                is connected to a branched sound input",
                graph.sound_processor(*spid).unwrap().friendly_name()
            ),
            SoundError::StaticNotSynchronous(spid) => format!(
                "The static processor {} is connected to a non-synchronous input",
                graph.sound_processor(*spid).unwrap().friendly_name()
            ),

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
