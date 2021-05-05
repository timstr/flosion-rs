use crate::sound::soundinput::SoundInputId;
use crate::sound::soundsource::{SoundSource, SoundSourceId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::rc::{Rc, Weak};

enum NodeId {
    SoundInputId(SoundInputId),
    SoundSourceId(SoundSourceId),
}

struct SoundGraphData {
    sound_sources: HashMap<SoundSourceId, Box<dyn SoundSource>>,
    next_id: usize,
}

impl SoundGraphData {
    pub fn new() -> SoundGraphData {
        SoundGraphData {
            sound_sources: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn sound_source(&self, ss_id: SoundSourceId) -> &dyn SoundSource {
        match self.sound_sources.get(&ss_id) {
            Some(&b) => &*b,
            None => panic!(),
        }
    }

    pub fn next_sound_source_id(&mut self) -> SoundSourceId {
        let i = self.next_ss_id.clone();
        self.next_ss_id.id += 1;
        i
    }

    pub fn next_sound_input_id(&mut self) -> SoundInputId {
        let i = self.next_si_id.clone();
        self.next_si_id.id += 1;
        i
    }
}

struct SoundGraph {
    data: Rc<RefCell<SoundGraphData>>,
}

struct SoundGraphRef {
    data: Weak<RefCell<SoundGraphData>>,
}

impl SoundGraph {
    pub fn new() -> SoundGraph {
        let data = SoundGraphData::new();
        SoundGraph {
            data: Rc::new(RefCell::new(data)),
        }
    }
    pub fn as_ref(&self) -> SoundGraphRef {
        SoundGraphRef {
            data: Rc::downgrade(&self.data),
        }
    }
    pub fn sound_source(&self, ss_id: SoundSourceId) -> &dyn SoundSource {
        self.data_ref().sound_source(ss_id)
    }

    fn data_ref<'a>(&'a self) -> impl Deref<Target = SoundGraphData> + 'a {
        self.data.borrow()
    }
    fn data_ref_mut<'a>(&'a self) -> impl DerefMut<Target = SoundGraphData> + 'a {
        self.data.borrow_mut()
    }
    pub fn next_sound_input_id(&mut self) -> SoundInputId {
        self.data_ref_mut().next_sound_input_id()
    }
    pub fn next_sound_source_id(&mut self) -> SoundSourceId {
        self.data_ref_mut().next_sound_source_id()
    }
}

impl SoundGraphRef {
    fn get(&self) -> SoundGraph {
        match self.data.upgrade() {
            Some(rc) => SoundGraph { data: rc },
            None => panic!(),
        }
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
