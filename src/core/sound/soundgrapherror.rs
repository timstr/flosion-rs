use super::{
    path::SoundPath, soundinput::SoundInputId, expression::SoundExpressionId,
    expressionargument::SoundExpressionArgumentId, soundprocessor::SoundProcessorId,
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
    BadSoundInputKeyIndex(SoundInputId, usize),
    SoundInputOccupied {
        input_id: SoundInputId,
        current_target: SoundProcessorId,
    },
    SoundInputUnoccupied(SoundInputId),
    CircularDependency {
        cycle: SoundPath,
    },
    StaticTooManyStates(SoundProcessorId),
    StaticNotSynchronous(SoundProcessorId),
    ArgumentIdTaken(SoundExpressionArgumentId),
    ArgumentNotFound(SoundExpressionArgumentId),
    BadArgumentInit(SoundExpressionArgumentId),
    BadArgumentCleanup(SoundExpressionArgumentId),
    ExpressionIdTaken(SoundExpressionId),
    BadExpressionInit(SoundExpressionId),
    BadExpressionCleanup(SoundExpressionId),
    ExpressionNotFound(SoundExpressionId),
    ParameterAlreadyBound {
        input_id: SoundExpressionId,
        target: SoundExpressionArgumentId,
    },
    ParameterNotBound {
        input_id: SoundExpressionId,
        target: SoundExpressionArgumentId,
    },
    StateNotInScope {
        bad_dependencies: Vec<(SoundExpressionArgumentId, SoundExpressionId)>,
    },
}
