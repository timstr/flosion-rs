#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Hash)]
pub struct SoundInputId {
    id: usize,
}

pub struct SoundInputData {
    target_id: Option<SoundSourceId>,
    own_id: SoundInputId,
    graph: SoundGraphRef,
}

impl SoundInputData {
    fn new(parent_sound_source: &mut SoundSource, graph: SoundGraphRef) -> SoundInputData {
        SoundInputData {
            target_id: None,
            onw,
        }
    }
}

impl SoundInputData {
    pub fn new(parent_sound_source: &dyn SoundSource, graph: SoundGraphRef) -> SoundInputData {
        SoundInputData {
            target_id: None,
            own_id: graph.get().next_sound_input_id(),
            graph: graph,
        }
    }
    pub fn get_next_chunk(&self, context: &SoundContext, chunk: &mut SoundChunk) {
        match &self.target_id {
            Some(ssi) => context
                .graph()
                .sound_source(*ssi)
                .get_next_chunk(context, chunk),
            _ => chunk.silence(),
        }
    }
}

pub struct SoundInput {
    data: Rc<RefCell<SoundInputData>>,
}

pub struct SoundInputRef {
    data: Weak<RefCell<SoundInputData>>,
}
