use std::sync::Arc;

use crate::{core::uniqueid::IdGenerator, ui_core::arguments::ParsedArguments};

use super::{
    expression::SoundExpressionId,
    expressionargument::{
        InputTimeExpressionArgument, ProcessorTimeExpressionArgument, SoundExpressionArgumentId,
        SoundExpressionArgumentOwner,
    },
    sounderror::SoundError,
    soundgraphdata::{
        SoundExpressionArgumentData, SoundInputBranchId, SoundInputData, SoundProcessorData,
    },
    soundgraphtopology::SoundGraphTopology,
    soundinput::{InputOptions, SoundInputId},
    soundprocessor::{
        DynamicSoundProcessor, DynamicSoundProcessorHandle, DynamicSoundProcessorWithId,
        SoundProcessorId, StaticSoundProcessor, StaticSoundProcessorHandle,
        StaticSoundProcessorWithId,
    },
    soundprocessortools::SoundProcessorTools,
};

/// Convenience struct for passing all sound graph id
/// generators around as a whole
pub(crate) struct SoundGraphIdGenerators {
    pub sound_processor: IdGenerator<SoundProcessorId>,
    pub sound_input: IdGenerator<SoundInputId>,
    pub expression_argument: IdGenerator<SoundExpressionArgumentId>,
    pub expression: IdGenerator<SoundExpressionId>,
}

impl SoundGraphIdGenerators {
    pub(crate) fn new() -> SoundGraphIdGenerators {
        SoundGraphIdGenerators {
            sound_processor: IdGenerator::new(),
            sound_input: IdGenerator::new(),
            expression_argument: IdGenerator::new(),
            expression: IdGenerator::new(),
        }
    }
}

/// Creates a new static sound processor using its StaticSoundProcessor::new()
/// method, adds it to the topology, and returns a handle to the processor.
pub(crate) fn build_static_sound_processor<T: StaticSoundProcessor>(
    topo: &mut SoundGraphTopology,
    idgens: &mut SoundGraphIdGenerators,
    args: ParsedArguments,
) -> Result<StaticSoundProcessorHandle<T>, SoundError> {
    let id = idgens.sound_processor.next_id();

    // Every sound processor gets a 'time' expression argument
    let time_data = SoundExpressionArgumentData::new(
        idgens.expression_argument.next_id(),
        Arc::new(ProcessorTimeExpressionArgument::new(id)),
        SoundExpressionArgumentOwner::SoundProcessor(id),
    );

    // Add a new processor data item to the topology,
    // but without the processor instance. This allows
    // the processor's topology to be modified within
    // the processor's new() method, e.g. to add inputs.
    let data = SoundProcessorData::new_empty(id);
    topo.add_sound_processor(data)?;

    // The tools which the processor can use to give itself
    // new inputs, etc
    let tools = SoundProcessorTools::new(id, topo, idgens);

    // construct the actual processor instance by its
    // concrete type
    let processor = T::new(tools, args).map_err(|_| SoundError::BadProcessorInit(id))?;

    // wrap the processor in a type-erased Arc
    let processor = Arc::new(StaticSoundProcessorWithId::new(
        processor,
        id,
        time_data.id(),
    ));
    let processor2 = Arc::clone(&processor);

    // add the missing processor instance to the
    // newly created processor data in the topology
    topo.sound_processor_mut(id)
        .unwrap()
        .set_processor(processor);

    // Add the 'time' expression argument
    topo.add_expression_argument(time_data)?;

    Ok(StaticSoundProcessorHandle::new(processor2))
}

/// Creates a new dynamic sound processor using its DynamicSoundProcessor::new()
/// method, adds it to the topology, and returns a handle to the processor.
pub(crate) fn build_dynamic_sound_processor<T: DynamicSoundProcessor>(
    topo: &mut SoundGraphTopology,
    idgens: &mut SoundGraphIdGenerators,
    args: ParsedArguments,
) -> Result<DynamicSoundProcessorHandle<T>, SoundError> {
    let id = idgens.sound_processor.next_id();

    // Every sound processor gets a 'time' expression argument
    let time_data = SoundExpressionArgumentData::new(
        idgens.expression_argument.next_id(),
        Arc::new(ProcessorTimeExpressionArgument::new(id)),
        SoundExpressionArgumentOwner::SoundProcessor(id),
    );

    // Add a new processor data item to the topology,
    // but without the processor instance. This allows
    // the processor's topology to be modified within
    // the processor's new() method, e.g. to add inputs.
    let data = SoundProcessorData::new_empty(id);
    topo.add_sound_processor(data)?;

    // The tools which the processor can use to give itself
    // new inputs, etc
    let tools = SoundProcessorTools::new(id, topo, idgens);

    // construct the actual processor instance by its
    // concrete type
    let processor = T::new(tools, args).map_err(|_| SoundError::BadProcessorInit(id))?;

    // wrap the processor in a type-erased Arc
    let processor = Arc::new(DynamicSoundProcessorWithId::new(
        processor,
        id,
        time_data.id(),
    ));
    let processor2 = Arc::clone(&processor);

    // add the missing processor instance to the
    // newly created processor data in the topology
    topo.sound_processor_mut(id)
        .unwrap()
        .set_processor(processor);

    // Add the 'time' expression argument
    topo.add_expression_argument(time_data)?;

    Ok(DynamicSoundProcessorHandle::new(processor2))
}

pub(crate) fn build_sound_input(
    topo: &mut SoundGraphTopology,
    idgens: &mut SoundGraphIdGenerators,
    owner_processor_id: SoundProcessorId,
    options: InputOptions,
    branches: Vec<SoundInputBranchId>,
) -> SoundInputId {
    let id = idgens.sound_input.next_id();

    let time_data = SoundExpressionArgumentData::new(
        idgens.expression_argument.next_id(),
        Arc::new(InputTimeExpressionArgument::new(id)),
        SoundExpressionArgumentOwner::SoundInput(id),
    );

    let input_data = SoundInputData::new(id, options, branches, owner_processor_id, time_data.id());

    topo.add_sound_input(input_data).unwrap();

    topo.add_expression_argument(time_data).unwrap();

    id
}
