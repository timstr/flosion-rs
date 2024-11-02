use std::{cell::RefCell, collections::HashMap, rc::Rc};

use eframe::egui;
use hashstash::{Order, Stashable, Stasher, UnstashError, Unstashable, Unstasher};

use crate::core::sound::{
    soundgraph::SoundGraph, soundgraphid::SoundObjectId, soundprocessor::SoundProcessorId,
};

use super::{
    arguments::ParsedArguments,
    object_ui::{random_object_color, ObjectUiState},
    stashing::UiUnstashingContext,
};

struct SoundObjectUiData {
    state: Rc<RefCell<dyn ObjectUiState>>,
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

    pub(super) fn set_object_data(
        &mut self,
        id: SoundObjectId,
        state: Rc<RefCell<dyn ObjectUiState>>,
    ) {
        self.data.insert(
            id,
            SoundObjectUiData {
                state,
                color: random_object_color(),
            },
        );
    }

    pub(super) fn get_object_data(&self, id: SoundObjectId) -> Rc<RefCell<dyn ObjectUiState>> {
        Rc::clone(&self.data.get(&id).unwrap().state)
    }

    pub(super) fn get_object_color(&self, id: SoundObjectId) -> egui::Color32 {
        self.data.get(&id).unwrap().color
    }

    pub(super) fn cleanup(&mut self, graph: &SoundGraph) {
        self.data.retain(|i, _| match i {
            SoundObjectId::Sound(spid) => graph.sound_processors().contains_key(spid),
        });
    }

    #[cfg(debug_assertions)]
    pub(crate) fn check_invariants(&self, graph: &SoundGraph) {
        let mut good = true;
        for i in graph.sound_processors().keys() {
            if !self.data.contains_key(&i.into()) {
                println!("Sound processor {} does not have a ui state", i.value());
                good = false;
            }
        }
        for i in self.data.keys() {
            match i {
                SoundObjectId::Sound(i) => {
                    if !graph.sound_processors().contains_key(i) {
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
        assert!(good);
    }
}

impl Stashable for SoundObjectUiStates {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.array_of_proxy_objects(
            self.data.iter(),
            |(object_id, ui_data), stasher| {
                object_id.stash(stasher);
                stasher.object_proxy(|stasher| ui_data.state.borrow().stash(stasher));
                stasher.u8(ui_data.color.r());
                stasher.u8(ui_data.color.g());
                stasher.u8(ui_data.color.b());
                stasher.u8(ui_data.color.a());
            },
            Order::Unordered,
        );
    }
}

impl Unstashable<UiUnstashingContext<'_>> for SoundObjectUiStates {
    fn unstash(
        unstasher: &mut Unstasher<UiUnstashingContext>,
    ) -> Result<SoundObjectUiStates, UnstashError> {
        let mut data = HashMap::new();
        unstasher.array_of_proxy_objects(|unstasher| {
            let proc_id = SoundProcessorId::new(unstasher.u64()? as _);

            let proc = unstasher
                .context()
                .sound_graph()
                .sound_processor(proc_id)
                .unwrap()
                .as_graph_object();

            let proc_ui = unstasher
                .context()
                .factories()
                .sound_uis()
                .get(proc.get_dynamic_type());

            let ui_state = proc_ui
                .make_ui_state(proc, &ParsedArguments::new_empty())
                .unwrap();

            unstasher.object_proxy_inplace_with_context(
                |unstasher| ui_state.borrow_mut().unstash_inplace(unstasher),
                (),
            )?;

            let color = egui::Color32::from_rgba_premultiplied(
                unstasher.u8()?,
                unstasher.u8()?,
                unstasher.u8()?,
                unstasher.u8()?,
            );

            data.insert(
                proc_id.into(),
                SoundObjectUiData {
                    state: ui_state,
                    color,
                },
            );

            Ok(())
        })?;

        Ok(SoundObjectUiStates { data })
    }
}
