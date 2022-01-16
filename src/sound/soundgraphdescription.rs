use std::collections::HashMap;

use super::{
    connectionerror::ConnectionError,
    numbersource::{NumberSourceId, NumberSourceOwner},
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
    owner: NumberSourceOwner,
}

pub struct SoundGraphDescription {
    sound_processors: HashMap<SoundProcessorId, SoundProcessorDescription>,
    sound_inputs: HashMap<SoundInputId, SoundInputDescription>,
}

impl SoundGraphDescription {
    pub fn new(
        sound_processors: HashMap<SoundProcessorId, SoundProcessorDescription>,
        sound_inputs: HashMap<SoundInputId, SoundInputDescription>,
    ) -> SoundGraphDescription {
        SoundGraphDescription {
            sound_processors,
            sound_inputs,
        }
    }

    pub fn find_error(&self) -> Option<ConnectionError> {
        if self.contains_cycles() {
            return Some(ConnectionError::CircularDependency);
        }
        if let Some(err) = self.validate_graph() {
            return Some(err);
        }
        None
    }

    pub fn add_connection(
        &mut self,
        input_id: SoundInputId,
        processor_id: SoundProcessorId,
    ) -> Option<ConnectionError> {
        let proc_desc = self.sound_processors.get_mut(&processor_id);
        if proc_desc.is_none() {
            return Some(ConnectionError::ProcessorNotFound);
        }

        let input_desc = self.sound_inputs.get_mut(&input_id);
        if input_desc.is_none() {
            return Some(ConnectionError::InputNotFound);
        }
        let input_desc = input_desc.unwrap();

        if let Some(prev_target) = input_desc.target {
            if prev_target == processor_id {
                return Some(ConnectionError::NoChange);
            }
            return Some(ConnectionError::InputOccupied);
        }

        input_desc.target = Some(processor_id);
        return None;
    }

    pub fn remove_connection(&mut self, input_id: SoundInputId) -> Option<ConnectionError> {
        let input_desc = self.sound_inputs.get_mut(&input_id);
        if input_desc.is_none() {
            return Some(ConnectionError::InputNotFound);
        }
        let input_desc = input_desc.unwrap();

        let processor_id = match input_desc.target {
            Some(pid) => pid,
            None => return Some(ConnectionError::NoChange),
        };

        let proc_desc = self.sound_processors.get_mut(&processor_id);
        if proc_desc.is_none() {
            return Some(ConnectionError::ProcessorNotFound);
        }

        if input_desc.target.is_none() {
            return Some(ConnectionError::NoChange);
        }

        input_desc.target = Some(processor_id);
        return None;
    }

    pub fn contains_cycles(&self) -> bool {
        fn dfs_find_cycle(
            processor_id: SoundProcessorId,
            visited: &mut Vec<SoundProcessorId>,
            path: &mut Vec<SoundProcessorId>,
            graph_desription: &SoundGraphDescription,
        ) -> bool {
            // If the current path already contains this processor, there is a cycle
            if path.contains(&processor_id) {
                return true;
            }
            if !visited.contains(&processor_id) {
                visited.push(processor_id)
            }
            path.push(processor_id);
            let mut found_cycle = false;
            let proc_desc = graph_desription
                .sound_processors
                .get(&processor_id)
                .unwrap();
            for input_id in &proc_desc.inputs {
                let input_desc = graph_desription.sound_inputs.get(&input_id).unwrap();
                if let Some(target) = input_desc.target {
                    if dfs_find_cycle(target, visited, path, graph_desription) {
                        found_cycle = true;
                        break;
                    }
                }
            }
            assert_eq!(path[path.len() - 1], processor_id);
            path.pop();
            found_cycle
        }
        let mut visited: Vec<SoundProcessorId> = vec![];
        let mut path: Vec<SoundProcessorId> = vec![];
        loop {
            assert_eq!(path.len(), 0);
            let proc_to_visit = self
                .sound_processors
                .iter()
                .find(|(pid, pdesc)| !visited.contains(&pid));
            match proc_to_visit {
                None => break false,
                Some((pid, pdesc)) => {
                    if dfs_find_cycle(*pid, &mut visited, &mut path, &self) {
                        break true;
                    }
                }
            }
        }
    }

    pub fn validate_graph(&self) -> Option<ConnectionError> {
        fn visit(
            proc_id: SoundProcessorId,
            states_to_add: usize,
            is_realtime: bool,
            graph_desription: &SoundGraphDescription,
            init: bool,
        ) -> Option<ConnectionError> {
            assert!(states_to_add != 0);
            let proc_desc = graph_desription.sound_processors.get(&proc_id).unwrap();
            if proc_desc.is_static {
                if states_to_add > 1 {
                    return Some(ConnectionError::StaticTooManyStates);
                }
                if !is_realtime {
                    return Some(ConnectionError::StaticNotRealtime);
                }
                if !init {
                    return None;
                }
            }
            for input_id in &proc_desc.inputs {
                let input_desc = graph_desription.sound_inputs.get(&input_id).unwrap();
                if let Some(t) = input_desc.target {
                    if let Some(err) = visit(
                        t,
                        states_to_add * input_desc.num_keys,
                        is_realtime && input_desc.options.realtime,
                        graph_desription,
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
}
