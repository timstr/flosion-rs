use super::{
    argument::ProcessorArgumentLocation, expression::ProcessorExpressionLocation,
    soundgraph::SoundGraph, soundinput::SoundInputLocation, soundprocessor::SoundProcessorId,
};

#[derive(Debug, Eq, PartialEq)]
pub enum SoundError {
    ProcessorNotFound(SoundProcessorId),
    SoundInputNotFound(SoundInputLocation),
    CircularDependency,
    ConnectionNotIsochronic(SoundInputLocation),
    StateNotInScope {
        bad_dependencies: Vec<(ProcessorArgumentLocation, ProcessorExpressionLocation)>,
    },
}

impl SoundError {
    pub(crate) fn explain(&self, graph: &SoundGraph) -> String {
        match self {
            SoundError::ProcessorNotFound(spid) => {
                format!("A processor with id #{} could not be found", spid.value())
            }
            SoundError::SoundInputNotFound(loc) => {
                format!(
                    "A sound input with id #{} on processor #{} could not be found",
                    loc.input().value(),
                    loc.processor().value()
                )
            }
            SoundError::CircularDependency => "The graph contains a cycle ".to_string(),
            SoundError::ConnectionNotIsochronic(loc) => format!(
                "Sound input #{} of processor {} is connected to a processor which is logically
                static, but the input is not isochronic",
                loc.input().value(),
                graph
                    .sound_processor(loc.processor())
                    .unwrap()
                    .as_graph_object()
                    .friendly_name()
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
