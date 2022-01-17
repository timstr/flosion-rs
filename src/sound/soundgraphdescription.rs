use std::collections::HashMap;

use super::{
    connectionerror::{ConnectionError, NumberConnectionError, SoundConnectionError},
    numberinput::{NumberInputId, NumberInputOwner},
    numbersource::{NumberSourceId, NumberSourceOwner},
    path::{NumberPath, SoundPath},
    soundinput::{InputOptions, SoundInputId},
    soundprocessor::SoundProcessorId,
};

pub struct SoundInputDescription {
    id: SoundInputId,
    options: InputOptions,
    num_keys: usize,
    target: Option<SoundProcessorId>,
}

impl SoundInputDescription {
    pub fn new(
        id: SoundInputId,
        options: InputOptions,
        num_keys: usize,
        target: Option<SoundProcessorId>,
    ) -> SoundInputDescription {
        SoundInputDescription {
            id,
            options,
            num_keys,
            target,
        }
    }
}

pub struct SoundProcessorDescription {
    id: SoundProcessorId,
    is_static: bool,
    inputs: Vec<SoundInputId>,
}

impl SoundProcessorDescription {
    pub fn new(
        id: SoundProcessorId,
        is_static: bool,
        inputs: Vec<SoundInputId>,
    ) -> SoundProcessorDescription {
        SoundProcessorDescription {
            id,
            is_static,
            inputs,
        }
    }
}

pub struct NumberSourceDescription {
    id: NumberSourceId,
    inputs: Vec<NumberInputId>,
    owner: NumberSourceOwner,
}

impl NumberSourceDescription {
    pub fn new(
        id: NumberSourceId,
        inputs: Vec<NumberInputId>,
        owner: NumberSourceOwner,
    ) -> NumberSourceDescription {
        NumberSourceDescription { id, inputs, owner }
    }
}

pub struct NumberInputDescription {
    id: NumberInputId,
    target: Option<NumberSourceId>,
    owner: NumberInputOwner,
}

impl NumberInputDescription {
    pub fn new(
        id: NumberInputId,
        target: Option<NumberSourceId>,
        owner: NumberInputOwner,
    ) -> NumberInputDescription {
        NumberInputDescription { id, target, owner }
    }
}

pub struct SoundGraphDescription {
    sound_processors: HashMap<SoundProcessorId, SoundProcessorDescription>,
    sound_inputs: HashMap<SoundInputId, SoundInputDescription>,
    number_sources: HashMap<NumberSourceId, NumberSourceDescription>,
    number_inputs: HashMap<NumberInputId, NumberInputDescription>,
}

impl SoundGraphDescription {
    pub fn new(
        sound_processors: HashMap<SoundProcessorId, SoundProcessorDescription>,
        sound_inputs: HashMap<SoundInputId, SoundInputDescription>,
        number_sources: HashMap<NumberSourceId, NumberSourceDescription>,
        number_inputs: HashMap<NumberInputId, NumberInputDescription>,
    ) -> SoundGraphDescription {
        SoundGraphDescription {
            sound_processors,
            sound_inputs,
            number_sources,
            number_inputs,
        }
    }

    pub fn find_error(&self) -> Option<ConnectionError> {
        if let Some(path) = self.find_sound_cycle() {
            return Some(ConnectionError::Sound(
                SoundConnectionError::CircularDependency { cycle: path },
            ));
        }
        if let Some(err) = self.validate_sound_connections() {
            return Some(ConnectionError::Sound(err));
        }
        if let Some(path) = self.find_number_cycle() {
            return Some(ConnectionError::Number(
                NumberConnectionError::CircularDependency { cycle: path },
            ));
        }
        if let Some(err) = self.validate_number_connections() {
            return Some(ConnectionError::Number(err));
        }
        None
    }

    pub fn add_connection(
        &mut self,
        input_id: SoundInputId,
        processor_id: SoundProcessorId,
    ) -> Option<SoundConnectionError> {
        let proc_desc = self.sound_processors.get_mut(&processor_id);
        if proc_desc.is_none() {
            return Some(SoundConnectionError::ProcessorNotFound(processor_id));
        }

        let input_desc = self.sound_inputs.get_mut(&input_id);
        if input_desc.is_none() {
            return Some(SoundConnectionError::InputNotFound(input_id));
        }
        let input_desc = input_desc.unwrap();

        if let Some(current_target) = input_desc.target {
            if current_target == processor_id {
                return Some(SoundConnectionError::NoChange);
            }
            return Some(SoundConnectionError::InputOccupied {
                input_id,
                current_target,
            });
        }

        input_desc.target = Some(processor_id);
        return None;
    }

    pub fn remove_connection(&mut self, input_id: SoundInputId) -> Option<SoundConnectionError> {
        let input_desc = self.sound_inputs.get_mut(&input_id);
        if input_desc.is_none() {
            return Some(SoundConnectionError::InputNotFound(input_id));
        }
        let input_desc = input_desc.unwrap();

        let processor_id = match input_desc.target {
            Some(pid) => pid,
            None => return Some(SoundConnectionError::NoChange),
        };

        let proc_desc = self.sound_processors.get_mut(&processor_id);
        if proc_desc.is_none() {
            return Some(SoundConnectionError::ProcessorNotFound(processor_id));
        }

        if input_desc.target.is_none() {
            return Some(SoundConnectionError::NoChange);
        }

        input_desc.target = Some(processor_id);
        return None;
    }

    pub fn find_sound_cycle(&self) -> Option<SoundPath> {
        fn dfs_find_cycle(
            processor_id: SoundProcessorId,
            visited: &mut Vec<SoundProcessorId>,
            path: &mut SoundPath,
            graph_desription: &SoundGraphDescription,
        ) -> Option<SoundPath> {
            if !visited.contains(&processor_id) {
                visited.push(processor_id)
            }
            let proc_desc = graph_desription
                .sound_processors
                .get(&processor_id)
                .unwrap();
            for input_id in &proc_desc.inputs {
                // If the input has already been visited, there is a cycle
                if path.contains_input(*input_id) {
                    let idx = path
                        .connections
                        .iter()
                        .position(|(_, siid)| *siid == *input_id)
                        .unwrap();
                    let cycle_connections = path.connections.split_off(idx);
                    return Some(SoundPath::new(cycle_connections));
                }
                let input_desc = graph_desription.sound_inputs.get(&input_id).unwrap();
                if let Some(target) = input_desc.target {
                    path.push(target, *input_id);
                    if let Some(path) = dfs_find_cycle(target, visited, path, graph_desription) {
                        return Some(path);
                    }
                    path.pop();
                }
            }
            None
        }

        let mut visited: Vec<SoundProcessorId> = vec![];
        let mut path: SoundPath = SoundPath::new(Vec::new());

        loop {
            assert_eq!(path.connections.len(), 0);
            let proc_to_visit = self
                .sound_processors
                .keys()
                .find(|pid| !visited.contains(&pid));
            match proc_to_visit {
                None => break None,
                Some(pid) => {
                    if let Some(path) = dfs_find_cycle(*pid, &mut visited, &mut path, &self) {
                        break Some(path);
                    }
                }
            }
        }
    }

    pub fn validate_sound_connections(&self) -> Option<SoundConnectionError> {
        fn visit(
            proc_id: SoundProcessorId,
            states_to_add: usize,
            is_realtime: bool,
            graph_description: &SoundGraphDescription,
            init: bool,
        ) -> Option<SoundConnectionError> {
            debug_assert!(states_to_add != 0);
            let proc_desc = graph_description.sound_processors.get(&proc_id).unwrap();
            if proc_desc.is_static {
                if states_to_add > 1 {
                    return Some(SoundConnectionError::StaticTooManyStates(proc_id));
                }
                if !is_realtime {
                    return Some(SoundConnectionError::StaticNotRealtime(proc_id));
                }
                if !init {
                    return None;
                }
            }
            for input_id in &proc_desc.inputs {
                let input_desc = graph_description.sound_inputs.get(&input_id).unwrap();
                if let Some(t) = input_desc.target {
                    if let Some(err) = visit(
                        t,
                        states_to_add * input_desc.num_keys,
                        is_realtime && input_desc.options.realtime,
                        graph_description,
                        false,
                    ) {
                        return Some(err);
                    }
                }
            }
            None
        }
        for proc_desc in self.sound_processors.values() {
            if proc_desc.is_static {
                if let Some(err) = visit(proc_desc.id, 1, true, &self, true) {
                    return Some(err);
                }
            }
        }
        None
    }

    pub fn find_number_cycle(&self) -> Option<NumberPath> {
        // TODO
        panic!()
    }

    pub fn validate_number_connections(&self) -> Option<NumberConnectionError> {
        // TODO
        panic!()
    }
}
