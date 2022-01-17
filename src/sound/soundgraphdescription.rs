use std::collections::HashMap;

use super::{
    connectionerror::{ConnectionError, NumberConnectionError, SoundConnectionError},
    numberinput::{NumberInputId, NumberInputOwner},
    numbersource::{NumberSourceId, NumberSourceOwner},
    path::{NumberPath, SoundPath},
    soundinput::{InputOptions, SoundInputId},
    soundprocessor::SoundProcessorId,
};

#[derive(Copy, Clone)]
enum StateOwner {
    SoundInput(SoundInputId),
    SoundProcessor(SoundProcessorId),
}

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
            input_id: SoundInputId,
            visited: &mut Vec<SoundInputId>,
            path: &mut SoundPath,
            graph_description: &SoundGraphDescription,
        ) -> Option<SoundPath> {
            if !visited.contains(&input_id) {
                visited.push(input_id);
            }
            // If the input has already been visited, there is a cycle
            if path.contains_input(input_id) {
                return Some(path.trim_until_input(input_id));
            }
            let input_desc = graph_description.sound_inputs.get(&input_id).unwrap();
            let target_id = match input_desc.target {
                Some(spid) => spid,
                _ => return None,
            };
            let proc_desc = graph_description.sound_processors.get(&target_id).unwrap();
            path.push(target_id, input_id);
            for target_proc_input in &proc_desc.inputs {
                if let Some(path) =
                    dfs_find_cycle(*target_proc_input, visited, path, graph_description)
                {
                    return Some(path);
                }
            }
            path.pop();
            None
        }

        let mut visited: Vec<SoundInputId> = Vec::new();
        let mut path: SoundPath = SoundPath::new(Vec::new());

        loop {
            assert_eq!(path.connections.len(), 0);
            let input_to_visit = self.sound_inputs.keys().find(|pid| !visited.contains(&pid));
            match input_to_visit {
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
        fn dfs_find_cycle(
            input_id: NumberInputId,
            visited: &mut Vec<NumberInputId>,
            path: &mut NumberPath,
            graph_description: &SoundGraphDescription,
        ) -> Option<NumberPath> {
            if !visited.contains(&input_id) {
                visited.push(input_id);
            }
            // If the input has already been visited, there is a cycle
            if path.contains_input(input_id) {
                return Some(path.trim_until_input(input_id));
            }
            let input_desc = graph_description.number_inputs.get(&input_id).unwrap();
            let target_id = match input_desc.target {
                Some(spid) => spid,
                _ => return None,
            };
            let source_desc = graph_description.number_sources.get(&target_id).unwrap();
            path.push(target_id, input_id);
            for target_proc_input in &source_desc.inputs {
                if let Some(path) =
                    dfs_find_cycle(*target_proc_input, visited, path, graph_description)
                {
                    return Some(path);
                }
            }
            path.pop();
            None
        }

        let mut visited: Vec<NumberInputId> = Vec::new();
        let mut path: NumberPath = NumberPath::new(Vec::new());

        loop {
            assert_eq!(path.connections.len(), 0);
            let input_to_visit = self
                .number_inputs
                .keys()
                .find(|pid| !visited.contains(&pid));
            match input_to_visit {
                None => break None,
                Some(pid) => {
                    if let Some(path) = dfs_find_cycle(*pid, &mut visited, &mut path, &self) {
                        break Some(path);
                    }
                }
            }
        }
    }

    fn state_reachable_from(&self, state_to_reach: StateOwner, owner: StateOwner) -> bool {
        // TODO
        panic!()
    }

    pub fn validate_number_connections(&self) -> Option<NumberConnectionError> {
        fn dfs_find_unreachable_state(
            input_id: NumberInputId,
            visited: &mut Vec<NumberInputId>,
            path: &mut NumberPath,
            state_to_reach: StateOwner,
            graph_description: &SoundGraphDescription,
        ) -> Option<NumberConnectionError> {
            if !visited.contains(&input_id) {
                visited.push(input_id);
            }
            let input_desc = graph_description.number_inputs.get(&input_id).unwrap();
            let target_id = match input_desc.target {
                Some(spid) => spid,
                _ => return None,
            };
            let target_desc = graph_description.number_sources.get(&target_id).unwrap();
            let target_owner: Option<StateOwner> = match target_desc.owner {
                NumberSourceOwner::Nothing => None,
                NumberSourceOwner::SoundInput(iid) => Some(StateOwner::SoundInput(iid)),
                NumberSourceOwner::SoundProcessor(pid) => Some(StateOwner::SoundProcessor(pid)),
            };
            if let Some(o) = target_owner {
                debug_assert!(
                    target_desc.inputs.is_empty(),
                    "Stateful number sources with their own inputs are not yet supported"
                );
                if !graph_description.state_reachable_from(state_to_reach, o) {
                    // return Some((state_to_reach, path, o));
                    return Some(NumberConnectionError::StateNotInScope { path: path.clone() });
                }
            }
            for target_source_input in &target_desc.inputs {
                path.push(target_id, input_id);
                if let Some(x) = dfs_find_unreachable_state(
                    *target_source_input,
                    visited,
                    path,
                    state_to_reach,
                    graph_description,
                ) {
                    return Some(x);
                }
                path.pop();
            }
            None
        }

        let mut visited: Vec<NumberInputId> = Vec::new();
        let mut path: NumberPath = NumberPath::new(Vec::new());

        loop {
            assert_eq!(path.connections.len(), 0);
            let input_to_visit;
            let state_owner;
            {
                let input = self
                    .number_inputs
                    .iter()
                    .find(|(iid, desc)| !visited.contains(&iid) && desc.owner.is_stateful());
                let (iid, desc) = match input {
                    Some(i) => i,
                    None => break None,
                };
                input_to_visit = *iid;
                state_owner = match desc.owner {
                    NumberInputOwner::NumberSource(_) => panic!(),
                    NumberInputOwner::SoundInput(iid) => StateOwner::SoundInput(iid),
                    NumberInputOwner::SoundProcessor(pid) => StateOwner::SoundProcessor(pid),
                };
            }
            if let Some(x) = dfs_find_unreachable_state(
                input_to_visit,
                &mut visited,
                &mut path,
                state_owner,
                self,
            ) {
                break Some(x);
            }
        }
    }
}
