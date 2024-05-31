use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[cfg(debug_assertions)]
use std::any::type_name;

use eframe::{egui, epaint::ecolor};

use crate::core::sound::{
    soundgraphid::SoundObjectId, soundgraphtopology::SoundGraphTopology,
    expression::SoundExpressionId,
};

use super::{
    graph_ui::{ObjectUiData, ObjectUiState},
    numbergraphui::NumberGraphUi,
    numbergraphuistate::NumberObjectUiStates,
    object_ui::Color,
    object_ui_states::AnyObjectUiState,
    soundgraphui::SoundGraphUi,
    soundgraphuicontext::SoundGraphUiContext,
    soundgraphuistate::SoundGraphUiState,
    ui_factory::UiFactory,
};

pub struct AnySoundObjectUiData {
    id: SoundObjectId,
    state: RefCell<Box<dyn AnyObjectUiState>>,
    color: Color,
}

impl ObjectUiData for AnySoundObjectUiData {
    type GraphUi = SoundGraphUi;
    type ConcreteType<'a, T: ObjectUiState> = SoundObjectUiData<'a, T>;

    type RequiredData = Color;

    fn new<S: ObjectUiState>(id: SoundObjectId, state: S, data: Self::RequiredData) -> Self {
        AnySoundObjectUiData {
            id,
            state: RefCell::new(Box::new(state)),
            color: data,
        }
    }

    fn downcast_with<
        T: ObjectUiState,
        F: FnOnce(SoundObjectUiData<'_, T>, &mut SoundGraphUiState, &mut SoundGraphUiContext<'_>),
    >(
        &self,
        ui_state: &mut SoundGraphUiState,
        ctx: &mut SoundGraphUiContext<'_>,
        f: F,
    ) {
        let mut state_mut = self.state.borrow_mut();
        #[cfg(debug_assertions)]
        {
            let actual_name = state_mut.get_language_type_name();
            let state_any = state_mut.as_mut_any();
            debug_assert!(
                state_any.is::<T>(),
                "AnySoundObjectUiData expected to receive state type {}, but got {:?} instead",
                type_name::<T>(),
                actual_name
            );
        }
        let state_any = state_mut.as_mut_any();
        let state = state_any.downcast_mut::<T>().unwrap();
        let color = ctx
            .object_states()
            .get_apparent_object_color(self.id, ui_state);
        f(SoundObjectUiData { state, color }, ui_state, ctx);
    }
}

pub struct SoundObjectUiData<'a, T: ObjectUiState> {
    pub state: &'a mut T,
    pub color: egui::Color32,
}

pub struct SoundObjectUiStates {
    data: HashMap<SoundObjectId, Rc<AnySoundObjectUiData>>,
    number_graph_object_states: HashMap<SoundExpressionId, NumberObjectUiStates>,
}

impl SoundObjectUiStates {
    pub(super) fn new() -> SoundObjectUiStates {
        SoundObjectUiStates {
            data: HashMap::new(),
            number_graph_object_states: HashMap::new(),
        }
    }

    pub(super) fn set_object_data(&mut self, id: SoundObjectId, state: AnySoundObjectUiData) {
        self.data.insert(id, Rc::new(state));
    }

    pub(super) fn get_object_data(&self, id: SoundObjectId) -> Rc<AnySoundObjectUiData> {
        Rc::clone(self.data.get(&id).unwrap())
    }

    pub(super) fn get_object_color(&self, id: SoundObjectId) -> egui::Color32 {
        self.data.get(&id).unwrap().color.color
    }

    pub(super) fn get_apparent_object_color(
        &self,
        id: SoundObjectId,
        ui_state: &SoundGraphUiState,
    ) -> egui::Color32 {
        let color = self.get_object_color(id);
        if ui_state.is_item_focused(id.into()) || ui_state.is_object_selected(id) {
            let mut hsva = ecolor::Hsva::from(color);
            hsva.v = 0.5 * (1.0 + hsva.a);
            hsva.into()
        } else {
            color
        }
    }

    pub(super) fn cleanup(&mut self, topo: &SoundGraphTopology) {
        self.data.retain(|i, _| match i {
            SoundObjectId::Sound(spid) => topo.sound_processors().contains_key(spid),
        });
        self.number_graph_object_states
            .retain(|i, _| topo.expressions().contains_key(i));
        for (niid, states) in &mut self.number_graph_object_states {
            let number_topo = topo
                .expression(*niid)
                .unwrap()
                .expression_graph()
                .topology();
            states.cleanup(number_topo);
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn check_invariants(&self, topo: &SoundGraphTopology) -> bool {
        use crate::core::uniqueid::UniqueId;

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
        // TODO: invariant check for number object states
        good
    }

    // pub(super) fn serialize(
    //     &self,
    //     serializer: &mut Serializer,
    //     subset: Option<&HashSet<ObjectId>>,
    //     idmap: &ForwardGraphIdMap,
    // ) {
    //     let is_selected = |id: ObjectId| match subset {
    //         Some(s) => s.get(&id).is_some(),
    //         None => true,
    //     };
    //     let mut s1 = serializer.subarchive();
    //     for (id, state) in &self.data {
    //         if !is_selected(*id) {
    //             continue;
    //         }
    //         serialize_object_id(*id, &mut s1, idmap);
    //         let color = u32::from_be_bytes([
    //             state.color.r(),
    //             state.color.g(),
    //             state.color.b(),
    //             state.color.a(),
    //         ]);
    //         s1.u32(color);
    //         let mut s2 = s1.subarchive();
    //         state.state.borrow().serialize(&mut s2);
    //     }
    // }

    // pub(super) fn deserialize(
    //     &mut self,
    //     deserializer: &mut Deserializer,
    //     idmap: &ReverseGraphIdMap,
    //     topology: &SoundGraphTopology,
    //     ui_factory: &UiFactory,
    // ) -> Result<(), ()> {
    //     let mut d1 = deserializer.subarchive()?;
    //     while !d1.is_empty() {
    //         let id = deserialize_object_id(&mut d1, idmap)?;
    //         let obj = match id {
    //             ObjectId::Sound(i) => match topology.sound_processor(i) {
    //                 Some(sp) => sp.instance_arc().as_graph_object(),
    //                 None => return Err(()),
    //             },
    //             ObjectId::Number(i) => match topology.number_source(i) {
    //                 Some(ns) => {
    //                     if let Some(o) = ns.instance_arc().as_graph_object() {
    //                         o
    //                     } else {
    //                         return Err(());
    //                     }
    //                 }
    //                 None => return Err(()),
    //             },
    //         };

    //         let color = match d1.u32() {
    //             Ok(i) => {
    //                 let [r, g, b, a] = i.to_be_bytes();
    //                 egui::Color32::from_rgba_premultiplied(r, g, b, a)
    //             }
    //             Err(_) => random_object_color(),
    //         };

    //         let d2 = d1.subarchive()?;
    //         let state = if let Ok(s) = ui_factory.create_state_from_archive(&obj, d2) {
    //             s
    //         } else {
    //             println!(
    //                 "Warning: could not deserialize state for object of type \"{}\"",
    //                 obj.get_type().name()
    //             );
    //             ui_factory.create_default_state(&obj)
    //         };
    //         self.set_object_data(id, state, color);
    //     }
    //     Ok(())
    // }

    pub(super) fn create_state_for(
        &mut self,
        object_id: SoundObjectId,
        topo: &SoundGraphTopology,
        ui_factory: &UiFactory<SoundGraphUi>,
        number_ui_factory: &UiFactory<NumberGraphUi>,
    ) {
        self.data.entry(object_id).or_insert_with(|| {
            let graph_object = topo.graph_object(object_id).unwrap();
            Rc::new(ui_factory.create_default_state(&graph_object))
        });
        match object_id {
            SoundObjectId::Sound(spid) => {
                let number_input_ids = topo.sound_processor(spid).unwrap().expressions();
                for niid in number_input_ids {
                    let number_topo = topo
                        .expression(*niid)
                        .unwrap()
                        .expression_graph()
                        .topology();
                    self.number_graph_object_states
                        .entry(*niid)
                        .or_insert_with(|| {
                            let mut states = NumberObjectUiStates::new();
                            for ns_data in number_topo.nodes().values() {
                                let graph_object = ns_data.instance_arc().as_graph_object();
                                let object_state =
                                    number_ui_factory.create_default_state(&graph_object);
                                states.set_object_data(ns_data.id(), object_state);
                            }
                            states
                        });
                }
            }
        }
    }

    pub(super) fn number_graph_object_state(
        &self,
        input_id: SoundExpressionId,
    ) -> &NumberObjectUiStates {
        self.number_graph_object_states.get(&input_id).unwrap()
    }

    pub(super) fn number_graph_object_state_mut(
        &mut self,
        input_id: SoundExpressionId,
    ) -> &mut NumberObjectUiStates {
        self.number_graph_object_states.get_mut(&input_id).unwrap()
    }
}
