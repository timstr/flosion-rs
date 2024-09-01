use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc};

use eframe::egui;

use crate::core::sound::{soundgraphid::SoundObjectId, soundgraphtopology::SoundGraphTopology};

use super::object_ui::random_object_color;

struct SoundObjectUiData {
    state: Rc<RefCell<dyn Any>>,
    color: egui::Color32,
}

pub struct SoundObjectUiStates {
    data: HashMap<SoundObjectId, SoundObjectUiData>,
}

impl SoundObjectUiStates {
    pub(super) fn new() -> SoundObjectUiStates {
        SoundObjectUiStates {
            data: HashMap::new(),
        }
    }

    pub(super) fn set_object_data(&mut self, id: SoundObjectId, state: Rc<RefCell<dyn Any>>) {
        self.data.insert(
            id,
            SoundObjectUiData {
                state,
                color: random_object_color(),
            },
        );
    }

    pub(super) fn get_object_data(&self, id: SoundObjectId) -> Rc<RefCell<dyn Any>> {
        Rc::clone(&self.data.get(&id).unwrap().state)
    }

    pub(super) fn get_object_color(&self, id: SoundObjectId) -> egui::Color32 {
        self.data.get(&id).unwrap().color
    }

    pub(super) fn cleanup(&mut self, topo: &SoundGraphTopology) {
        self.data.retain(|i, _| match i {
            SoundObjectId::Sound(spid) => topo.sound_processors().contains_key(spid),
        });
    }

    #[cfg(debug_assertions)]
    pub(crate) fn check_invariants(&self, topo: &SoundGraphTopology) -> bool {
        let mut good = true;
        for i in topo.sound_processors().keys() {
            if !self.data.contains_key(&i.into()) {
                println!("Sound processor {} does not have a ui state", i.value());
                good = false;
            }
        }
        for i in self.data.keys() {
            match i {
                SoundObjectId::Sound(i) => {
                    if !topo.sound_processors().contains_key(i) {
                        println!(
                            "A ui state exists for non-existent sound processor {}",
                            i.value()
                        );
                        good = false;
                    }
                }
            }
        }
        // TODO: invariant check for expression object states
        good
    }
}
