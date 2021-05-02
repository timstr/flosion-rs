use crate::sound::soundchunk::SoundChunk;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Hash)]
pub struct SoundSourceId {
    id: usize,
}

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Hash)]
pub struct SoundInputId {
    id: usize,
}

pub trait SoundSource {
    fn get_next_chunk(&self, context: &SoundContext, chunk: &mut SoundChunk);
    fn inputs(&self) -> Vec<SoundInputId>;
    fn id(&self) -> SoundSourceId;
}

pub struct SoundInput {
    target_id: Option<SoundSourceId>,
    own_id: SoundInputId,
}

impl SoundInput {
    pub fn new(parent_sound_source: &dyn SoundSource, graph: &mut SoundGraph) -> SoundInput {
        SoundInput {
            target_id: None,
            own_id: graph.next_sound_input_id(),
        }
    }
    pub fn get_next_chunk(&self, context: &SoundContext, chunk: &mut SoundChunk) {
        match &self.target_id {
            Some(ssi) => context.graph().get(*ssi).get_next_chunk(context, chunk),
            _ => chunk.silence(),
        }
    }
    pub fn connect(&mut self, target: SoundSourceId, graph: &SoundGraph) {}
    pub fn disconnect(&mut self) {}
    pub fn source(&self) -> Option<SoundSourceId> {
        self.target_id.clone()
    }
}

pub struct SoundGraph {
    nodes: HashMap<SoundSourceId, Box<dyn SoundSource>>,
    next_ss_id: SoundSourceId,
    next_si_id: SoundInputId,
}

impl SoundGraph {
    pub fn new() -> SoundGraph {
        SoundGraph {
            nodes: HashMap::new(),
            next_ss_id: SoundSourceId { id: 0 },
            next_si_id: SoundInputId { id: 0 },
        }
    }

    pub fn add(&mut self, node: Box<dyn SoundSource>) -> SoundSourceId {
        let id = self.next_sound_source_id();
        self.nodes.insert(id, node);
        // TODO: register all sound inputs
        id
    }

    pub fn get(&self, sound_source_id: SoundSourceId) -> &dyn SoundSource {
        let n = self.nodes.get(&sound_source_id).unwrap();
        n.deref()
    }

    pub fn remove(&mut self, node_id: SoundSourceId) {
        self.nodes.remove(&node_id).unwrap();
    }

    fn next_sound_source_id(&mut self) -> SoundSourceId {
        let i = self.next_ss_id.clone();
        self.next_ss_id.id += 1;
        i
    }

    fn next_sound_input_id(&mut self) -> SoundInputId {
        let i = self.next_si_id.clone();
        self.next_si_id.id += 1;
        i
    }
}

#[derive(Copy, Clone)]
pub struct StateIndex {
    index: usize,
    owner: SoundSourceId,
}

pub struct StatePath {
    path: Vec<StateIndex>,
}

pub struct SoundContext<'a> {
    parent_graph: &'a SoundGraph,
    state_path: StatePath,
}

impl<'a> SoundContext<'a> {
    pub fn graph(&'a self) -> &'a SoundGraph {
        self.parent_graph
    }
}

pub struct StateTable<T> {
    data: Vec<RefCell<T>>,
}

impl<T> StateTable<T> {
    pub fn get<'a>(&'a self, index: StateIndex) -> impl DerefMut<Target = T> + 'a {
        self.data[index.index].borrow_mut()
    }
}
