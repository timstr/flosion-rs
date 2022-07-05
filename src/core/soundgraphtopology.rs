use std::{collections::HashMap, sync::Arc};

use crate::core::{gridspan::GridSpan, soundgrapherror::SoundConnectionError};

use super::{
    key::TypeErasedKey,
    numberinput::{NumberInputHandle, NumberInputId, NumberInputOwner},
    numbersource::{NumberSource, NumberSourceId, NumberSourceOwner},
    soundgraphdata::{
        EngineNumberInputData, EngineNumberSourceData, EngineSoundInputData,
        EngineSoundProcessorData,
    },
    soundgraphdescription::{
        NumberInputDescription, NumberSourceDescription, SoundGraphDescription,
        SoundInputDescription, SoundProcessorDescription,
    },
    soundgrapherror::{NumberConnectionError, SoundGraphError},
    soundinput::{SoundInputId, SoundInputWrapper},
    soundprocessor::{SoundProcessorId, SoundProcessorWrapper},
};

#[derive(Copy, Clone)]
pub enum StateOperation {
    Insert,
    Erase,
}

#[derive(Clone)]
pub struct SoundGraphTopology {
    sound_processors: HashMap<SoundProcessorId, EngineSoundProcessorData>,
    sound_inputs: HashMap<SoundInputId, EngineSoundInputData>,
    number_sources: HashMap<NumberSourceId, EngineNumberSourceData>,
    number_inputs: HashMap<NumberInputId, EngineNumberInputData>,
}

impl SoundGraphTopology {
    pub fn new() -> SoundGraphTopology {
        SoundGraphTopology {
            sound_processors: HashMap::new(),
            sound_inputs: HashMap::new(),
            number_sources: HashMap::new(),
            number_inputs: HashMap::new(),
        }
    }

    pub fn sound_processors(&self) -> &HashMap<SoundProcessorId, EngineSoundProcessorData> {
        &self.sound_processors
    }

    pub fn sound_inputs(&self) -> &HashMap<SoundInputId, EngineSoundInputData> {
        &self.sound_inputs
    }

    pub fn number_sources(&self) -> &HashMap<NumberSourceId, EngineNumberSourceData> {
        &self.number_sources
    }

    pub fn number_inputs(&self) -> &HashMap<NumberInputId, EngineNumberInputData> {
        &self.number_inputs
    }

    pub fn add_sound_processor(&mut self, wrapper: Arc<dyn SoundProcessorWrapper>) {
        let processor_id = wrapper.id();
        debug_assert!(
            self.sound_processors.get(&processor_id).is_none(),
            "The processor id should not already be in use"
        );
        let data = EngineSoundProcessorData::new(wrapper, processor_id);
        self.sound_processors.insert(processor_id, data);
        // self.update_static_processor_cache();
    }

    pub fn remove_sound_processor(&mut self, processor_id: SoundProcessorId) {
        // disconnect all sound inputs from the processor
        let mut sound_inputs_to_disconnect: Vec<SoundInputId> = Vec::new();
        for (input_id, input_data) in self.sound_inputs.iter() {
            // if this input belongs to the sound processor, remove it
            if input_data.owner() == processor_id {
                sound_inputs_to_disconnect.push(*input_id)
            }
            // if this input is connected to the sound processor, remove it
            if let Some(target_id) = input_data.target() {
                if target_id == processor_id {
                    sound_inputs_to_disconnect.push(*input_id)
                }
            }
        }
        for input_id in sound_inputs_to_disconnect {
            self.disconnect_sound_input(input_id).unwrap();
        }

        // remove all sound inputs belonging to the processor
        let sound_inputs_to_remove = self
            .sound_processors
            .get(&processor_id)
            .unwrap()
            .inputs()
            .clone();
        for input_id in sound_inputs_to_remove {
            self.remove_sound_input(input_id);
        }

        // disconnect all number inputs from the sound processor
        let mut number_inputs_to_disconnect: Vec<NumberInputId> = Vec::new();
        for (input_id, input_data) in self.number_inputs.iter() {
            // if this number input belongs to the sound processor, disconnect it
            if let NumberInputOwner::SoundProcessor(spid) = input_data.owner() {
                if spid == processor_id {
                    number_inputs_to_disconnect.push(*input_id);
                }
            }
            // if this number input is connected to a number source belonging to
            // the sound processor, remove it
            if let Some(target) = input_data.target() {
                let target_data = self.number_sources.get(&target).unwrap();
                if let NumberSourceOwner::SoundProcessor(spid) = target_data.owner() {
                    if spid == processor_id {
                        number_inputs_to_disconnect.push(*input_id);
                    }
                }
            }
        }
        for input_id in number_inputs_to_disconnect {
            self.disconnect_number_input(input_id).unwrap();
        }

        {
            // remove all number inputs belonging to the processor
            for input_id in self
                .sound_processors
                .get(&processor_id)
                .unwrap()
                .number_inputs()
            {
                self.number_inputs.remove(&input_id).unwrap();
            }
        }

        // disconnect all number sources belonging to the processor
        for source_id in self
            .sound_processors
            .get(&processor_id)
            .unwrap()
            .number_sources()
        {
            self.number_sources.remove(&source_id).unwrap();
        }

        // remove the processor
        self.sound_processors.remove(&processor_id).unwrap();

        // self.update_static_processor_cache();
    }

    pub fn add_sound_input(
        &mut self,
        processor_id: SoundProcessorId,
        input: Arc<dyn SoundInputWrapper>,
    ) {
        debug_assert!(
            self.sound_processors
                .iter()
                .find_map(|(_, pd)| pd.inputs().iter().find(|i| **i == input.id()))
                .is_none(),
            "The input id should not already be associated with any sound processors"
        );
        debug_assert!(
            self.sound_inputs.get(&input.id()).is_none(),
            "The input id should not already be in use by a sound input"
        );
        let proc_data = self.sound_processors.get_mut(&processor_id).unwrap();
        proc_data.inputs_mut().push(input.id());
        let gs = GridSpan::new_contiguous(0, proc_data.wrapper().num_states());
        input.insert_states(gs);
        let input_data = EngineSoundInputData::new(input, processor_id);
        self.sound_inputs.insert(input_data.id(), input_data);
    }

    pub fn remove_sound_input(&mut self, input_id: SoundInputId) {
        let target;
        let owner;
        let number_sources_to_remove;
        {
            let input_data = self.sound_inputs.get(&input_id).unwrap();
            owner = input_data.owner();
            target = input_data.target();
            number_sources_to_remove = input_data.number_sources().clone();
        }
        if target.is_some() {
            self.disconnect_sound_input(input_id).unwrap();
        }
        for nsid in number_sources_to_remove {
            self.number_sources.remove(&nsid).unwrap();
        }
        let proc_data = self.sound_processors.get_mut(&owner).unwrap();
        proc_data.inputs_mut().retain(|iid| *iid != input_id);
        self.sound_inputs.remove(&input_id).unwrap();
        // self.update_static_processor_cache();
    }

    pub fn add_sound_input_key(&mut self, input_id: SoundInputId, key: TypeErasedKey) {
        let input_data = self.sound_inputs.get(&input_id).unwrap();
        let gs = input_data.input().insert_key(key);
        if let Some(proc_id) = input_data.target() {
            self.modify_states_recursively(proc_id, gs, input_id, StateOperation::Insert);
        }
    }

    pub fn remove_sound_input_key(&mut self, input_id: SoundInputId, key_index: usize) {
        let input_data = self.sound_inputs.get(&input_id).unwrap();
        let gs = input_data.input().erase_key(key_index);
        if let Some(proc_id) = input_data.target() {
            self.modify_states_recursively(proc_id, gs, input_id, StateOperation::Erase);
        }
    }

    fn modify_states_recursively(
        &mut self,
        proc_id: SoundProcessorId,
        dst_states: GridSpan,
        dst_iid: SoundInputId,
        operation: StateOperation,
    ) {
        let mut outbound_connections: Vec<(SoundProcessorId, GridSpan, SoundInputId)> = Vec::new();

        let proc_data = self.sound_processors.get_mut(&proc_id).unwrap();
        let proc = &mut proc_data.wrapper();
        let gs = match operation {
            StateOperation::Insert => proc.insert_dst_states(dst_iid, dst_states),
            StateOperation::Erase => proc.erase_dst_states(dst_iid, dst_states),
        };
        if proc.is_static() {
            return;
        }
        for i in proc_data.inputs() {
            let input_data = self.sound_inputs.get_mut(&i).unwrap();
            let gsi = match operation {
                StateOperation::Insert => input_data.input().insert_states(gs),
                StateOperation::Erase => input_data.input().erase_states(gs),
            };
            if let Some(pid) = input_data.target() {
                outbound_connections.push((pid, gsi, input_data.id()));
            };
        }

        for (pid, gsi, iid) in outbound_connections {
            Self::modify_states_recursively(self, pid, gsi, iid, operation);
        }
    }

    pub fn connect_sound_input(
        &mut self,
        input_id: SoundInputId,
        processor_id: SoundProcessorId,
    ) -> Result<(), SoundGraphError> {
        let mut desc = self.describe();
        debug_assert!(desc.find_error().is_none());

        if let Some(err) = desc.add_sound_connection(input_id, processor_id) {
            return Err(err.into());
        }

        if let Some(err) = desc.find_error() {
            return Err(err);
        }

        let input_data = self.sound_inputs.get_mut(&input_id);
        if input_data.is_none() {
            return Err(SoundConnectionError::InputNotFound(input_id).into());
        }
        let input_data = input_data.unwrap();
        if let Some(pid) = input_data.target() {
            if pid == processor_id {
                return Err(SoundConnectionError::NoChange.into());
            }
            return Err(SoundConnectionError::InputOccupied {
                input_id,
                current_target: pid,
            }
            .into());
        }
        input_data.set_target(Some(processor_id));

        {
            let proc_data = self.sound_processors.get_mut(&processor_id);
            if proc_data.is_none() {
                return Err(SoundConnectionError::ProcessorNotFound(processor_id).into());
            }
            let proc_data = proc_data.unwrap();
            proc_data.wrapper().add_dst(input_id);
        }

        let input_proc_states = self
            .sound_processors
            .get(&input_data.owner())
            .unwrap()
            .wrapper()
            .num_states();

        let input_keys = input_data.input().num_keys();

        self.modify_states_recursively(
            processor_id,
            GridSpan::new_contiguous(0, input_proc_states * input_keys),
            input_id,
            StateOperation::Insert,
        );

        // self.update_static_processor_cache();

        debug_assert!(self.describe().find_error().is_none());

        Ok(())
    }

    pub fn disconnect_sound_input(
        &mut self,
        input_id: SoundInputId,
    ) -> Result<(), SoundGraphError> {
        let mut desc = self.describe();
        debug_assert!(desc.find_error().is_none());

        if let Some(err) = desc.remove_sound_connection(input_id) {
            return Err(err.into());
        }

        if let Some(err) = desc.find_error() {
            return Err(err.into());
        }

        let input_data = self.sound_inputs.get_mut(&input_id);
        if input_data.is_none() {
            return Err(SoundConnectionError::InputNotFound(input_id).into());
        }
        let input_data = input_data.unwrap();
        let processor_id = match input_data.target() {
            Some(pid) => pid,
            None => return Ok(()),
        };

        input_data.set_target(None);

        let input_proc_states = self
            .sound_processors
            .get(&input_data.owner())
            .unwrap()
            .wrapper()
            .num_states();

        let input_keys = input_data.input().num_keys();

        self.modify_states_recursively(
            processor_id,
            GridSpan::new_contiguous(0, input_proc_states * input_keys),
            input_id,
            StateOperation::Erase,
        );

        {
            let proc_data = self.sound_processors.get_mut(&processor_id);
            if proc_data.is_none() {
                return Err(SoundConnectionError::ProcessorNotFound(processor_id).into());
            }
            let proc_data = proc_data.unwrap();
            proc_data.wrapper().remove_dst(input_id);
        }

        // self.update_static_processor_cache();

        debug_assert!(self.describe().find_error().is_none());

        Ok(())
    }

    pub fn add_number_source(
        &mut self,
        id: NumberSourceId,
        source: Arc<dyn NumberSource>,
        owner: NumberSourceOwner,
    ) {
        debug_assert!(self.number_sources.get(&id).is_none());
        let data = EngineNumberSourceData::new(id, source, owner, Vec::new());
        self.number_sources.insert(id, data);
        match owner {
            NumberSourceOwner::SoundProcessor(spid) => {
                let proc_data = self.sound_processors.get_mut(&spid).unwrap();
                debug_assert!(!proc_data.number_sources().contains(&id));
                proc_data.number_sources_mut().push(id);
            }
            NumberSourceOwner::SoundInput(siid) => {
                let input_data = self.sound_inputs.get_mut(&siid).unwrap();
                debug_assert!(!input_data.number_sources().contains(&id));
                input_data.number_sources_mut().push(id);
            }
            NumberSourceOwner::Nothing => (),
        }
    }

    pub fn remove_number_source(&mut self, source_id: NumberSourceId) {
        let mut inputs_to_disconnect: Vec<NumberInputId> = Vec::new();
        for (input_id, input_data) in self.number_inputs.iter() {
            // if this input belongs to the number source, disconnect it
            if let NumberInputOwner::NumberSource(nsid) = input_data.owner() {
                if nsid == source_id {
                    inputs_to_disconnect.push(*input_id);
                }
            }
            // if this input is connected to the number source, disconnect it
            if let Some(target) = input_data.target() {
                if target == source_id {
                    inputs_to_disconnect.push(*input_id);
                }
            }
        }
        for input_id in inputs_to_disconnect {
            self.disconnect_number_input(input_id).unwrap();
        }

        // remove all number inputs belonging to the source
        let number_inputs_to_remove = self
            .number_sources
            .get(&source_id)
            .unwrap()
            .inputs()
            .clone();
        for input_id in number_inputs_to_remove {
            Self::remove_number_input(self, input_id);
        }

        // remove the number source from its owner, if any
        match self.number_sources.get(&source_id).unwrap().owner() {
            NumberSourceOwner::SoundProcessor(spid) => {
                let proc_data = self.sound_processors.get_mut(&spid).unwrap();
                proc_data
                    .number_sources_mut()
                    .retain(|iid| *iid != source_id);
            }
            NumberSourceOwner::SoundInput(siid) => {
                let input_data = self.sound_inputs.get_mut(&siid).unwrap();
                input_data
                    .number_sources_mut()
                    .retain(|iid| *iid != source_id);
            }
            NumberSourceOwner::Nothing => (),
        }

        // remove the number source
        self.number_sources.remove(&source_id).unwrap();
    }

    pub fn add_number_input(&mut self, handle: NumberInputHandle) {
        let id = handle.id();
        let owner = handle.owner();
        debug_assert!(self.number_inputs.get(&id).is_none());

        let data = EngineNumberInputData::new(id, None, owner);
        self.number_inputs.insert(id, data);

        match owner {
            NumberInputOwner::SoundProcessor(spid) => {
                let proc_data = self.sound_processors.get_mut(&spid).unwrap();
                debug_assert!(!proc_data.number_inputs().contains(&id));
                proc_data.number_inputs_mut().push(id);
            }
            NumberInputOwner::NumberSource(nsid) => {
                let source_data = self.number_sources.get_mut(&nsid).unwrap();
                debug_assert!(!source_data.inputs().contains(&id));
                source_data.inputs_mut().push(id);
            }
        }
    }

    pub fn remove_number_input(&mut self, id: NumberInputId) {
        let target;
        let owner;
        {
            let input_data = self.number_inputs.get(&id).unwrap();
            target = input_data.target();
            owner = input_data.owner();
        }
        if target.is_some() {
            Self::disconnect_number_input(self, id).unwrap();
        }
        match owner {
            NumberInputOwner::SoundProcessor(spid) => {
                let proc_data = self.sound_processors.get_mut(&spid).unwrap();
                proc_data.number_inputs_mut().retain(|niid| *niid != id);
            }
            NumberInputOwner::NumberSource(nsid) => {
                let source_data = self.number_sources.get_mut(&nsid).unwrap();
                source_data.inputs_mut().retain(|niid| *niid != id);
            }
        }

        self.number_inputs.remove(&id);
    }

    pub fn connect_number_input(
        &mut self,
        input_id: NumberInputId,
        source_id: NumberSourceId,
    ) -> Result<(), NumberConnectionError> {
        let mut desc = self.describe();
        debug_assert!(desc.find_error().is_none());

        if let Some(err) = desc.add_number_connection(input_id, source_id) {
            return Err(err);
        }

        if let Some(err) = desc.find_error() {
            return Err(err.into_number().unwrap());
        }

        let input_data = match self.number_inputs.get_mut(&input_id) {
            Some(i) => i,
            None => return Err(NumberConnectionError::InputNotFound(input_id)),
        };

        if self.number_sources.get(&source_id).is_none() {
            return Err(NumberConnectionError::SourceNotFound(source_id));
        }

        if let Some(t) = input_data.target() {
            if t == source_id {
                return Err(NumberConnectionError::NoChange);
            }
            return Err(NumberConnectionError::InputOccupied(input_id, t));
        }

        input_data.set_target(Some(source_id));

        Ok(())
    }

    pub fn disconnect_number_input(
        &mut self,
        input_id: NumberInputId,
    ) -> Result<(), NumberConnectionError> {
        let mut desc = self.describe();
        debug_assert!(desc.find_error().is_none());

        if let Some(err) = desc.remove_number_connection(input_id) {
            return Err(err.into());
        }

        if let Some(err) = desc.find_error() {
            return Err(err.into_number().unwrap());
        }

        let input_data = match self.number_inputs.get_mut(&input_id) {
            Some(i) => i,
            None => return Err(NumberConnectionError::InputNotFound(input_id)),
        };

        input_data.set_target(None);

        Ok(())
    }

    pub fn describe(&self) -> SoundGraphDescription {
        let mut sound_processors = HashMap::<SoundProcessorId, SoundProcessorDescription>::new();
        for proc_data in self.sound_processors.values() {
            sound_processors.insert(
                proc_data.id(),
                SoundProcessorDescription::new(
                    proc_data.id(),
                    proc_data.wrapper().is_static(),
                    proc_data.inputs().clone(),
                    proc_data.number_sources().clone(),
                    proc_data.number_inputs().clone(),
                ),
            );
        }
        let mut sound_inputs = HashMap::<SoundInputId, SoundInputDescription>::new();
        for input_data in self.sound_inputs.values() {
            sound_inputs.insert(
                input_data.id(),
                SoundInputDescription::new(
                    input_data.id(),
                    input_data.input().options(),
                    input_data.input().num_keys(),
                    input_data.target(),
                    input_data.owner(),
                    input_data.number_sources().clone(),
                ),
            );
        }
        let mut number_sources = HashMap::<NumberSourceId, NumberSourceDescription>::new();
        for source_data in self.number_sources.values() {
            number_sources.insert(
                source_data.id(),
                NumberSourceDescription::new(
                    source_data.id(),
                    source_data.inputs().clone(),
                    source_data.owner(),
                ),
            );
        }
        let mut number_inputs = HashMap::<NumberInputId, NumberInputDescription>::new();
        for input_data in self.number_inputs.values() {
            number_inputs.insert(
                input_data.id(),
                NumberInputDescription::new(
                    input_data.id(),
                    input_data.target(),
                    input_data.owner(),
                ),
            );
        }
        SoundGraphDescription::new(
            sound_processors,
            sound_inputs,
            number_sources,
            number_inputs,
        )
    }
}
