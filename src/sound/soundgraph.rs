use crate::sound::id::{Id, IdGenerator};
use crate::sound::soundchunk::SoundChunk;
use std::collections::HashMap;
use std::ops::Deref;

struct SoundSourceTag;
struct SoundInputTag;
type SoundSourceId = Id<SoundSourceTag>;
type SoundInputId = Id<SoundInputTag>;

struct SoundSourceBase {}

pub trait SoundSource {
    fn get_next_chunk(&self, context: &SoundContext, chunk: &mut SoundChunk);
    fn inputs(&self) -> Vec<&SoundInput>;
    fn id(&self) -> SoundSourceId;
}

pub struct SoundInput {
    target_id: Option<SoundSourceId>,
    own_id: Option<SoundInputId>,
}

impl SoundInput {
    pub fn new(parent_sound_source: &dyn SoundSource) -> SoundInput {
        SoundInput {
            target_id: None,
            own_id: None,
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
    ss_id_gen: IdGenerator<SoundSourceTag>,
    si_id_gen: IdGenerator<SoundInputTag>,
}

impl SoundGraph {
    pub fn new() -> SoundGraph {
        SoundGraph {
            nodes: HashMap::new(),
            ss_id_gen: IdGenerator::<SoundInputTag>::new(),
            si_id_gen: IdGenerator::<SoundInputTag>::new(),
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
        let i = self.next_ssid.clone();
        self.next_ssid.id += 1;
        i
    }

    fn next_sound_input_id(&mut self) -> SoundInputId {
        let i = self.next_siid.clone();
        self.next_siid.id += 1;
        i
    }
}

struct StateIndex {
    index: usize,
    owner: SoundSourceId,
}

pub struct SoundContext<'a> {
    parent_graph: &'a SoundGraph,
}

impl<'a> SoundContext<'a> {
    pub fn graph(&'a self) -> &'a SoundGraph {
        self.parent_graph
    }
}
