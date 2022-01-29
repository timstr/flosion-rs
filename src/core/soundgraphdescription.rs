use std::collections::HashMap;

use super::{
    numberinput::{NumberInputId, NumberInputOwner},
    numbersource::{NumberSourceId, NumberSourceOwner},
    path::{NumberPath, SoundPath},
    soundgrapherror::{NumberConnectionError, SoundConnectionError, SoundGraphError},
    soundinput::{InputOptions, SoundInputId},
    soundprocessor::SoundProcessorId,
    soundstate::StateOwner,
};

pub struct SoundInputDescription {
    id: SoundInputId,
    options: InputOptions,
    num_keys: usize,
    target: Option<SoundProcessorId>,
    owner: SoundProcessorId,
    number_sources: Vec<NumberSourceId>,
}

impl SoundInputDescription {
    pub fn new(
        id: SoundInputId,
        options: InputOptions,
        num_keys: usize,
        target: Option<SoundProcessorId>,
        owner: SoundProcessorId,
        number_sources: Vec<NumberSourceId>,
    ) -> SoundInputDescription {
        SoundInputDescription {
            id,
            options,
            num_keys,
            target,
            owner,
            number_sources,
        }
    }
}

pub struct SoundProcessorDescription {
    id: SoundProcessorId,
    is_static: bool,
    inputs: Vec<SoundInputId>,
    number_sources: Vec<NumberSourceId>,
    number_inputs: Vec<NumberInputId>,
}

impl SoundProcessorDescription {
    pub fn new(
        id: SoundProcessorId,
        is_static: bool,
        inputs: Vec<SoundInputId>,
        number_sources: Vec<NumberSourceId>,
        number_inputs: Vec<NumberInputId>,
    ) -> SoundProcessorDescription {
        SoundProcessorDescription {
            id,
            is_static,
            inputs,
            number_sources,
            number_inputs,
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

    pub fn find_error(&self) -> Option<SoundGraphError> {
        self.check_missing_ids();

        if let Some(path) = self.find_sound_cycle() {
            return Some(SoundConnectionError::CircularDependency { cycle: path }.into());
        }
        if let Some(err) = self.validate_sound_connections() {
            return Some(SoundGraphError::Sound(err));
        }
        if let Some(path) = self.find_number_cycle() {
            return Some(NumberConnectionError::CircularDependency { cycle: path }.into());
        }
        let bad_dependencies = self.find_invalid_number_connections();
        if bad_dependencies.len() > 0 {
            return Some(NumberConnectionError::StateNotInScope { bad_dependencies }.into());
        }
        None
    }

    pub fn add_sound_connection(
        &mut self,
        input_id: SoundInputId,
        processor_id: SoundProcessorId,
    ) -> Option<SoundConnectionError> {
        if self.sound_processors.get_mut(&processor_id).is_none() {
            return Some(SoundConnectionError::ProcessorNotFound(processor_id));
        }
        let input_desc = match self.sound_inputs.get_mut(&input_id) {
            Some(i) => i,
            None => return Some(SoundConnectionError::InputNotFound(input_id)),
        };
        input_desc.target = Some(processor_id);
        None
    }

    pub fn remove_sound_connection(
        &mut self,
        input_id: SoundInputId,
    ) -> Option<SoundConnectionError> {
        let input_desc = match self.sound_inputs.get_mut(&input_id) {
            Some(i) => i,
            None => return Some(SoundConnectionError::InputNotFound(input_id)),
        };
        let processor_id = match input_desc.target {
            Some(t) => t,
            None => return Some(SoundConnectionError::NoChange),
        };
        if self.sound_processors.get(&processor_id).is_none() {
            return Some(SoundConnectionError::ProcessorNotFound(processor_id));
        }
        input_desc.target = Some(processor_id);
        None
    }

    pub fn add_number_connection(
        &mut self,
        input_id: NumberInputId,
        source_id: NumberSourceId,
    ) -> Option<NumberConnectionError> {
        if self.number_sources.get_mut(&source_id).is_none() {
            return Some(NumberConnectionError::SourceNotFound(source_id));
        }
        let input_desc = match self.number_inputs.get_mut(&input_id) {
            Some(i) => i,
            None => return Some(NumberConnectionError::InputNotFound(input_id)),
        };
        input_desc.target = Some(source_id);
        None
    }

    pub fn remove_number_connection(
        &mut self,
        input_id: NumberInputId,
    ) -> Option<NumberConnectionError> {
        let input_desc = match self.number_inputs.get_mut(&input_id) {
            Some(i) => i,
            None => return Some(NumberConnectionError::InputNotFound(input_id)),
        };
        let source_id = match input_desc.target {
            Some(t) => t,
            None => return Some(NumberConnectionError::NoChange),
        };
        if self.number_sources.get_mut(&source_id).is_none() {
            return Some(NumberConnectionError::SourceNotFound(source_id));
        }
        input_desc.target = Some(source_id);
        None
    }

    pub fn check_missing_ids(&self) {
        for sp in self.sound_processors.values() {
            // for each sound processor
            for i in &sp.inputs {
                // each sound input must exist and list the sound processor as its owner
                match self.sound_inputs.get(i) {
                    Some(idata) => {
                        if idata.owner != sp.id {
                            panic!(
                                "Sound processor {:?} has sound input {:?} listed as an\
                                input, but that input does not list the sound processor\
                                as its owner",
                                sp.id, i
                            );
                        }
                    }
                    None => panic!("1"),
                }
            }
            for i in &sp.number_inputs {
                // each number input must exist and list the sound processor as its owner
                match self.number_inputs.get(i) {
                    Some(idata) => {
                        if idata.owner != NumberInputOwner::SoundProcessor(sp.id) {
                            panic!("2");
                        }
                    }
                    None => panic!("3"),
                }
            }
            for s in &sp.number_sources {
                // each number source must exist and list the sound processor as its owner
                match self.number_sources.get(s) {
                    Some(sdata) => {
                        if sdata.owner != NumberSourceOwner::SoundProcessor(sp.id) {
                            panic!("4");
                        }
                    }
                    None => panic!("5"),
                }
            }
        }

        for si in self.sound_inputs.values() {
            // for each sound input
            match self.sound_processors.get(&si.owner) {
                // its owner must exist and list the input
                Some(sp) => {
                    if !sp.inputs.contains(&si.id) {
                        panic!("6");
                    }
                }
                None => panic!("7"),
            }
            for nsid in &si.number_sources {
                // each number source must exist and list the sound input as its owner
                match self.number_sources.get(nsid) {
                    Some(ns) => {
                        if ns.owner != NumberSourceOwner::SoundInput(si.id) {
                            panic!("8");
                        }
                    }
                    None => panic!("9"),
                }
            }
        }

        for ns in self.number_sources.values() {
            // for each number source
            for niid in &ns.inputs {
                // each number input must exist and list the number source as its owner
                match self.number_inputs.get(niid) {
                    Some(ni) => {
                        if ni.owner != NumberInputOwner::NumberSource(ns.id) {
                            panic!("10");
                        }
                    }
                    None => panic!("11"),
                }
            }
            match ns.owner {
                // if the number source has an owner, it must exist and list the number source
                NumberSourceOwner::SoundProcessor(spid) => match self.sound_processors.get(&spid) {
                    Some(sp) => {
                        if !sp.number_sources.contains(&ns.id) {
                            panic!("12");
                        }
                    }
                    None => panic!("13"),
                },
                NumberSourceOwner::SoundInput(siid) => match self.sound_inputs.get(&siid) {
                    Some(si) => {
                        if !si.number_sources.contains(&ns.id) {
                            panic!("14");
                        }
                    }
                    None => panic!("15"),
                },
                NumberSourceOwner::Nothing => (),
            }
        }

        for ni in self.number_inputs.values() {
            // for all number inputs
            if let Some(nsid) = &ni.target {
                // its target, if any, must exist
                if self.number_sources.get(nsid).is_none() {
                    panic!("16");
                }
            }
            match &ni.owner {
                // the number input's owner must exist and list the number input
                NumberInputOwner::SoundProcessor(spid) => match self.sound_processors.get(spid) {
                    Some(sp) => {
                        if !sp.number_inputs.contains(&ni.id) {
                            panic!("17");
                        }
                    }
                    None => panic!("18"),
                },
                NumberInputOwner::NumberSource(nsid) => match self.number_sources.get(nsid) {
                    Some(ns) => {
                        if !ns.inputs.contains(&ni.id) {
                            panic!("19");
                        }
                    }
                    None => panic!("20"),
                },
            }
        }
        // whew, made it
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

    fn state_owner_has_dependency(&self, owner: StateOwner, dependency: StateOwner) -> bool {
        if owner == dependency {
            return true;
        }
        match owner {
            StateOwner::SoundInput(siid) => {
                let input_desc = self.sound_inputs.get(&siid).unwrap();
                if let Some(spid) = input_desc.target {
                    return self
                        .state_owner_has_dependency(StateOwner::SoundProcessor(spid), dependency);
                }
                return false;
            }
            StateOwner::SoundProcessor(spid) => {
                let proc_desc = self.sound_processors.get(&spid).unwrap();
                for siid in &proc_desc.inputs {
                    if self.state_owner_has_dependency(StateOwner::SoundInput(*siid), dependency) {
                        return true;
                    }
                }
                return false;
            }
        }
    }

    pub fn find_all_stateful_dependencies_of(
        &self,
        input_id: NumberInputId,
    ) -> Vec<NumberSourceId> {
        fn dfs(
            input_id: NumberInputId,
            out_sources: &mut Vec<NumberSourceId>,
            graph_description: &SoundGraphDescription,
        ) {
            let input_desc = graph_description.number_inputs.get(&input_id).unwrap();
            if let Some(target_id) = input_desc.target {
                let target_desc = graph_description.number_sources.get(&target_id).unwrap();
                if target_desc.owner.is_stateful() {
                    out_sources.push(target_id);
                }
                for target_input_id in &target_desc.inputs {
                    dfs(*target_input_id, out_sources, graph_description);
                }
            }
        }

        let mut stateful_sources: Vec<NumberSourceId> = Vec::new();
        dfs(input_id, &mut stateful_sources, self);
        stateful_sources
    }

    pub fn find_all_stateful_dependents_of(&self, source: NumberSourceId) -> Vec<NumberInputId> {
        fn dfs(
            input_id: NumberInputId,
            source_id: NumberSourceId,
            graph: &SoundGraphDescription,
        ) -> bool {
            let input_desc = graph.number_inputs.get(&input_id).unwrap();
            if let Some(target_id) = input_desc.target {
                if target_id == source_id {
                    return true;
                }
                let target_desc = graph.number_sources.get(&target_id).unwrap();
                for target_input_id in &target_desc.inputs {
                    if dfs(*target_input_id, source_id, graph) {
                        return true;
                    }
                }
            }
            false
        }

        let mut stateful_dependents: Vec<NumberInputId> = Vec::new();
        for input_id in self.number_inputs.values().filter_map(|input_desc| {
            if input_desc.owner.is_stateful() {
                Some(input_desc.id)
            } else {
                None
            }
        }) {
            if dfs(input_id, source, self) {
                stateful_dependents.push(input_id);
            }
        }
        stateful_dependents
    }

    pub fn find_invalid_number_connections(&self) -> Vec<(NumberSourceId, NumberInputId)> {
        let mut bad_dependencies: Vec<(NumberSourceId, NumberInputId)> = Vec::new();

        for input_desc in self
            .number_inputs
            .values()
            .filter(|i| i.owner.is_stateful())
        {
            let stateful_sources = self.find_all_stateful_dependencies_of(input_desc.id);

            let input_owner = input_desc.owner.as_state_owner().unwrap();
            for ss in stateful_sources {
                let source_owner = self
                    .number_sources
                    .get(&ss)
                    .unwrap()
                    .owner
                    .as_state_owner()
                    .unwrap();
                if !self.state_owner_has_dependency(source_owner, input_owner) {
                    bad_dependencies.push((ss, input_desc.id));
                }
            }
        }

        return bad_dependencies;
    }
}
