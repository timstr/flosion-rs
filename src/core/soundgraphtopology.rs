use std::{collections::HashMap, sync::Arc};

use parking_lot::RwLock;

use crate::core::soundgrapherror::SoundConnectionError;

use super::{
    numberinput::{NumberInputHandle, NumberInputId, NumberInputOwner},
    numbersource::{
        NumberSource, NumberSourceId, NumberSourceOwner, PureNumberSource, PureNumberSourceHandle,
        StateNumberSourceHandle,
    },
    numbersourcetools::NumberSourceTools,
    soundchunk::SoundChunk,
    soundgraphdata::{
        EngineNumberInputData, EngineNumberSourceData, EngineSoundInputData,
        EngineSoundProcessorData,
    },
    soundgraphdescription::{
        NumberInputDescription, NumberSourceDescription, SoundGraphDescription,
        SoundInputDescription, SoundProcessorDescription,
    },
    soundgrapherror::{NumberConnectionError, SoundGraphError},
    soundinput::{InputOptions, SoundInputId},
    soundprocessor::{SoundProcessor, SoundProcessorHandle, SoundProcessorId},
    soundprocessortools::SoundProcessorTools,
    statetree::{NodeAllocator, ProcessorNodeWrapper},
    uniqueid::IdGenerator,
};

pub struct StaticProcessorCache {
    processor_id: SoundProcessorId,
    cached_output: RwLock<Option<SoundChunk>>,
    tree: RwLock<Box<dyn ProcessorNodeWrapper>>,
}

impl StaticProcessorCache {
    pub fn new(
        processor_id: SoundProcessorId,
        tree: Box<dyn ProcessorNodeWrapper>,
    ) -> StaticProcessorCache {
        StaticProcessorCache {
            processor_id,
            cached_output: RwLock::new(None),
            tree: RwLock::new(tree),
        }
    }

    pub fn processor_id(&self) -> SoundProcessorId {
        self.processor_id
    }

    pub fn output(&self) -> &RwLock<Option<SoundChunk>> {
        &self.cached_output
    }

    pub fn tree(&self) -> &RwLock<Box<dyn ProcessorNodeWrapper>> {
        &self.tree
    }
}

pub struct SoundGraphTopology {
    sound_processors: HashMap<SoundProcessorId, EngineSoundProcessorData>,
    sound_inputs: HashMap<SoundInputId, EngineSoundInputData>,
    number_sources: HashMap<NumberSourceId, EngineNumberSourceData>,
    number_inputs: HashMap<NumberInputId, EngineNumberInputData>,

    sound_processor_idgen: IdGenerator<SoundProcessorId>,
    sound_input_idgen: IdGenerator<SoundInputId>,
    number_source_idgen: IdGenerator<NumberSourceId>,
    number_input_idgen: IdGenerator<NumberInputId>,

    static_processors: Vec<StaticProcessorCache>,
}

impl SoundGraphTopology {
    pub fn new() -> SoundGraphTopology {
        SoundGraphTopology {
            sound_processors: HashMap::new(),
            sound_inputs: HashMap::new(),
            number_sources: HashMap::new(),
            number_inputs: HashMap::new(),

            sound_processor_idgen: IdGenerator::new(),
            sound_input_idgen: IdGenerator::new(),
            number_source_idgen: IdGenerator::new(),
            number_input_idgen: IdGenerator::new(),

            static_processors: Vec::new(),
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

    pub fn static_processors(&self) -> &Vec<StaticProcessorCache> {
        &self.static_processors
    }

    pub fn add_sound_processor<T: SoundProcessor>(&mut self) -> SoundProcessorHandle<T> {
        let processor_id = self.sound_processor_idgen.next_id();
        let data = EngineSoundProcessorData::new_without_processor(processor_id);
        self.sound_processors.insert(processor_id, data);
        let processor;
        {
            let tools = SoundProcessorTools::new(processor_id, self);
            processor = Arc::new(T::new(tools));
        }
        self.sound_processors
            .get_mut(&processor_id)
            .unwrap()
            .set_processor(Arc::<T>::clone(&processor));
        self.update_static_processor_cache();
        SoundProcessorHandle::new(processor_id, processor)
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
            self.disconnect_sound_input_impl(input_id).unwrap();
        }

        // remove all sound inputs belonging to the processor
        let sound_inputs_to_remove = self
            .sound_processors
            .get(&processor_id)
            .unwrap()
            .sound_inputs()
            .clone();
        for input_id in sound_inputs_to_remove {
            self.remove_sound_input_impl(input_id);
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

        self.update_static_processor_cache();
    }

    pub fn add_sound_input(
        &mut self,
        processor_id: SoundProcessorId,
        options: InputOptions,
        num_keys: usize,
    ) -> SoundInputId {
        let input_id = self.sound_input_idgen.next_id();
        let proc_data = self.sound_processors.get_mut(&processor_id).unwrap();
        proc_data.sound_inputs_mut().push(input_id);
        let input_data = EngineSoundInputData::new(input_id, options, num_keys, processor_id);
        self.sound_inputs.insert(input_data.id(), input_data);
        input_id
    }

    pub fn remove_sound_input(&mut self, input_id: SoundInputId) {
        let res = self.remove_sound_input_impl(input_id);
        self.update_static_processor_cache();
        res
    }

    fn remove_sound_input_impl(&mut self, input_id: SoundInputId) {
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
        proc_data.sound_inputs_mut().retain(|iid| *iid != input_id);
        self.sound_inputs.remove(&input_id).unwrap();
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

        self.update_static_processor_cache();

        debug_assert!(self.describe().find_error().is_none());

        Ok(())
    }

    pub fn disconnect_sound_input(
        &mut self,
        input_id: SoundInputId,
    ) -> Result<(), SoundGraphError> {
        let res = self.disconnect_sound_input_impl(input_id);
        self.update_static_processor_cache();
        res
    }

    fn disconnect_sound_input_impl(
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
        input_data.set_target(None);

        debug_assert!(self.describe().find_error().is_none());

        Ok(())
    }

    pub fn add_pure_number_source<T: PureNumberSource>(&mut self) -> PureNumberSourceHandle<T> {
        let id = self.number_source_idgen.next_id();
        let data = EngineNumberSourceData::new(id, None, NumberSourceOwner::Nothing);
        self.number_sources.insert(id, data);
        let tools = NumberSourceTools::new(id, self);
        let source = Arc::new(T::new(tools));
        let source2 = Arc::clone(&source);
        self.number_sources.get_mut(&id).unwrap().set_source(source);
        PureNumberSourceHandle::new(id, source2)
    }

    pub fn add_state_number_source(
        &mut self,
        source: Arc<dyn NumberSource>,
        owner: NumberSourceOwner,
    ) -> StateNumberSourceHandle {
        let id = self.number_source_idgen.next_id();
        let data = EngineNumberSourceData::new(id, Some(source), owner);
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
            NumberSourceOwner::Nothing => panic!("A state number source must have an owner"),
        }
        StateNumberSourceHandle::new(id, owner)
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

    pub fn add_number_input(&mut self, owner: NumberInputOwner) -> NumberInputHandle {
        let id = self.number_input_idgen.next_id();

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
        NumberInputHandle::new(id, owner)
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

    pub(super) fn make_state_tree_for(
        &self,
        input_id: SoundInputId,
    ) -> Option<Box<dyn ProcessorNodeWrapper>> {
        let input_data = self.sound_inputs.get(&input_id).unwrap();
        match input_data.target() {
            Some(proc_id) => {
                let allocator = NodeAllocator::new(proc_id, self);
                Some(
                    self.sound_processors
                        .get(&proc_id)
                        .unwrap()
                        .processor()
                        .make_node(&allocator),
                )
            }
            None => None,
        }
    }

    pub fn describe(&self) -> SoundGraphDescription {
        let mut sound_processors = HashMap::<SoundProcessorId, SoundProcessorDescription>::new();
        for proc_data in self.sound_processors.values() {
            sound_processors.insert(proc_data.id(), proc_data.describe());
        }
        let mut sound_inputs = HashMap::<SoundInputId, SoundInputDescription>::new();
        for input_data in self.sound_inputs.values() {
            sound_inputs.insert(input_data.id(), input_data.describe());
        }
        let mut number_sources = HashMap::<NumberSourceId, NumberSourceDescription>::new();
        for source_data in self.number_sources.values() {
            number_sources.insert(source_data.id(), source_data.describe());
        }
        let mut number_inputs = HashMap::<NumberInputId, NumberInputDescription>::new();
        for input_data in self.number_inputs.values() {
            number_inputs.insert(input_data.id(), input_data.describe());
        }
        SoundGraphDescription::new(
            sound_processors,
            sound_inputs,
            number_sources,
            number_inputs,
        )
    }

    fn update_static_processor_cache(&mut self) {
        let mut remaining_static_proc_ids: Vec<SoundProcessorId> = self
            .sound_processors
            .values()
            .filter_map(|proc_data| {
                if proc_data.processor().is_static() {
                    Some(proc_data.id())
                } else {
                    None
                }
            })
            .collect();
        fn depends_on_remaining_procs(
            proc_id: SoundProcessorId,
            remaining: &Vec<SoundProcessorId>,
            topology: &SoundGraphTopology,
        ) -> bool {
            let proc_data = topology.sound_processors().get(&proc_id).unwrap();
            for input_id in proc_data.sound_inputs() {
                let input_data = topology.sound_inputs().get(&input_id).unwrap();
                if let Some(target_proc_id) = input_data.target() {
                    if remaining
                        .iter()
                        .find(|pid| **pid == target_proc_id)
                        .is_some()
                    {
                        return true;
                    }
                    if depends_on_remaining_procs(target_proc_id, remaining, topology) {
                        return true;
                    }
                }
            }
            return false;
        }

        self.static_processors.clear();

        loop {
            let next_avail_proc = remaining_static_proc_ids.iter().position(|pid| {
                !depends_on_remaining_procs(*pid, &remaining_static_proc_ids, self)
            });
            match next_avail_proc {
                Some(idx) => {
                    let pid = remaining_static_proc_ids.remove(idx);
                    let proc_data = self.sound_processors.get(&pid).unwrap();
                    debug_assert!(proc_data.processor().is_static());
                    let allocator = NodeAllocator::new(pid, self);
                    let tree = proc_data.processor().make_node(&allocator);
                    self.static_processors
                        .push(StaticProcessorCache::new(pid, tree))
                }
                None => break,
            }
        }
    }
}
