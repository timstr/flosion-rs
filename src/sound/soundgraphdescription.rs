use super::{
    connectionerror::ConnectionError,
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
    inputs: Vec<SoundInputDescription>,
}

impl SoundProcessorDescription {
    pub fn new(
        id: SoundProcessorId,
        is_static: bool,
        inputs: Vec<SoundInputDescription>,
    ) -> SoundProcessorDescription {
        SoundProcessorDescription {
            id,
            is_static,
            inputs,
        }
    }
}

pub struct SoundGraphDescription {
    processors: Vec<SoundProcessorDescription>,
}

impl SoundGraphDescription {
    pub fn new(processors: Vec<SoundProcessorDescription>) -> SoundGraphDescription {
        SoundGraphDescription { processors }
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
        if self
            .processors
            .iter()
            .find(|p| p.id == processor_id)
            .is_none()
        {
            return Some(ConnectionError::ProcessorNotFound);
        }
        for p in &mut self.processors {
            let i = match p.inputs.iter_mut().find(|i| i.id == input_id) {
                None => continue,
                Some(i) => i,
            };
            if let Some(prev_proc) = i.target {
                if prev_proc == processor_id {
                    return Some(ConnectionError::NoChange);
                }
                return Some(ConnectionError::InputOccupied);
            }
            i.target = Some(processor_id);
            return None;
        }
        Some(ConnectionError::InputNotFound)
    }

    pub fn remove_connection(&mut self, input_id: SoundInputId) -> Option<ConnectionError> {
        for p in &mut self.processors {
            let i = match p.inputs.iter_mut().find(|i| i.id == input_id) {
                None => continue,
                Some(i) => i,
            };
            assert_eq!(i.id, input_id);
            if i.target.is_none() {
                return Some(ConnectionError::NoChange);
            }
            i.target = None;
            return None;
        }
        Some(ConnectionError::InputNotFound)
    }

    pub fn contains_cycles(&self) -> bool {
        fn dfs_find_cycle(
            id: SoundProcessorId,
            visited: &mut Vec<SoundProcessorId>,
            path: &mut Vec<SoundProcessorId>,
            processors: &Vec<SoundProcessorDescription>,
        ) -> bool {
            // If the current path already contains this processor, there is a cycle
            if path.contains(&id) {
                return true;
            }
            if !visited.contains(&id) {
                visited.push(id)
            }
            path.push(id);
            let mut found_cycle = false;
            let p = processors.iter().find(|spd| spd.id == id).unwrap();
            for i in p.inputs.iter().filter_map(|input| input.target) {
                if dfs_find_cycle(i, visited, path, processors) {
                    found_cycle = true;
                    break;
                }
            }
            assert_eq!(path[path.len() - 1], id);
            path.pop();
            found_cycle
        }
        let mut visited: Vec<SoundProcessorId> = vec![];
        let mut path: Vec<SoundProcessorId> = vec![];
        loop {
            assert_eq!(path.len(), 0);
            match self.processors.iter().find(|p| !visited.contains(&p.id)) {
                None => break false,
                Some(i) => {
                    if dfs_find_cycle(i.id, &mut visited, &mut path, &self.processors) {
                        break true;
                    }
                }
            }
        }
    }

    pub fn validate_graph(&self) -> Option<ConnectionError> {
        fn visit(
            proc: SoundProcessorId,
            states_to_add: usize,
            is_realtime: bool,
            procs: &Vec<SoundProcessorDescription>,
            init: bool,
        ) -> Option<ConnectionError> {
            assert!(states_to_add != 0);
            let p = procs.iter().find(|spd| spd.id == proc).unwrap();
            if p.is_static {
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
            for i in &p.inputs {
                if let Some(t) = i.target {
                    if let Some(err) = visit(
                        t,
                        states_to_add * i.num_keys,
                        is_realtime && i.options.realtime,
                        procs,
                        false,
                    ) {
                        return Some(err);
                    }
                }
            }
            None
        }
        for i in &self.processors {
            if i.is_static {
                if let Some(err) = visit(i.id, 1, true, &self.processors, true) {
                    return Some(err);
                }
            }
        }
        None
    }
}
