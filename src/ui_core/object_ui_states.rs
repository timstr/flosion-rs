use std::{
    any::{type_name, Any},
    collections::{HashMap, HashSet},
};

use eframe::egui;

use crate::core::{
    graphobject::{GraphId, GraphObjectHandle, ObjectId},
    graphserialization::{
        deserialize_object_id, serialize_object_id, ForwardGraphIdMap, ReverseGraphIdMap,
    },
    numbersource::NumberSourceOwner,
    serialization::{Deserializer, Serializable, Serializer},
    soundgraphtopology::SoundGraphTopology,
};

use super::{object_ui::random_object_color, ui_factory::UiFactory};

pub trait AnyObjectUiState: 'static {
    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;
    fn get_language_type_name(&self) -> &'static str;
    fn serialize(&self, serializer: &mut Serializer);
}

impl<T: 'static + Serializable> AnyObjectUiState for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<T>()
    }

    fn serialize(&self, serializer: &mut Serializer) {
        serializer.object(self);
    }
}

pub struct AnyObjectUiData {
    state: Box<dyn AnyObjectUiState>,
    color: egui::Color32,
}

impl AnyObjectUiData {
    pub(crate) fn state(&self) -> &dyn AnyObjectUiState {
        &*self.state
    }

    pub(crate) fn state_mut(&mut self) -> &mut dyn AnyObjectUiState {
        &mut *self.state
    }

    pub(crate) fn color(&self) -> egui::Color32 {
        self.color
    }
}

pub struct ObjectUiStates {
    data: HashMap<ObjectId, AnyObjectUiData>,
}

impl ObjectUiStates {
    pub(super) fn new() -> ObjectUiStates {
        ObjectUiStates {
            data: HashMap::new(),
        }
    }

    pub(super) fn set_object_data(
        &mut self,
        id: ObjectId,
        state: Box<dyn AnyObjectUiState>,
        color: egui::Color32,
    ) {
        self.data.insert(id, AnyObjectUiData { state, color });
    }

    pub(super) fn get_object_data(&mut self, id: ObjectId) -> &mut AnyObjectUiData {
        &mut *self.data.get_mut(&id).unwrap()
    }

    pub(super) fn cleanup(&mut self, remaining_ids: &HashSet<GraphId>) {
        self.data
            .retain(|i, _| remaining_ids.contains(&(*i).into()));
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
        for (i, ns) in topo.number_sources() {
            if ns.owner() == NumberSourceOwner::Nothing {
                if !self.data.contains_key(&i.into()) {
                    println!("Pure number source {} does not have a ui state", i.value());
                    good = false;
                }
            }
        }
        for i in self.data.keys() {
            match i {
                ObjectId::Sound(i) => {
                    if !topo.sound_processors().contains_key(i) {
                        println!(
                            "A ui state exists for non-existent sound processor {}",
                            i.value()
                        );
                        good = false;
                    }
                }
                ObjectId::Number(i) => {
                    if !topo.number_sources().contains_key(i) {
                        println!(
                            "A ui state exists for non-existent number source {}",
                            i.value()
                        );
                        good = false;
                    }
                }
            }
        }
        good
    }

    pub(super) fn serialize(
        &self,
        serializer: &mut Serializer,
        subset: Option<&HashSet<ObjectId>>,
        idmap: &ForwardGraphIdMap,
    ) {
        let is_selected = |id: ObjectId| match subset {
            Some(s) => s.get(&id).is_some(),
            None => true,
        };
        let mut s1 = serializer.subarchive();
        for (id, state) in &self.data {
            if !is_selected(*id) {
                continue;
            }
            serialize_object_id(*id, &mut s1, idmap);
            let color = u32::from_be_bytes([
                state.color.r(),
                state.color.g(),
                state.color.b(),
                state.color.a(),
            ]);
            s1.u32(color);
            let mut s2 = s1.subarchive();
            state.state.serialize(&mut s2);
        }
    }

    pub(super) fn deserialize(
        &mut self,
        deserializer: &mut Deserializer,
        idmap: &ReverseGraphIdMap,
        topology: &SoundGraphTopology,
        ui_factory: &UiFactory,
    ) -> Result<(), ()> {
        let mut d1 = deserializer.subarchive()?;
        while !d1.is_empty() {
            let id = deserialize_object_id(&mut d1, idmap)?;
            let obj = match id {
                ObjectId::Sound(i) => match topology.sound_processor(i) {
                    Some(sp) => sp.instance_arc().as_graph_object(),
                    None => return Err(()),
                },
                ObjectId::Number(i) => match topology.number_source(i) {
                    Some(ns) => {
                        if let Some(o) = ns.instance_arc().as_graph_object() {
                            o
                        } else {
                            return Err(());
                        }
                    }
                    None => return Err(()),
                },
            };

            let color = match d1.u32() {
                Ok(i) => {
                    let [r, g, b, a] = i.to_be_bytes();
                    egui::Color32::from_rgba_premultiplied(r, g, b, a)
                }
                Err(_) => random_object_color(),
            };

            let d2 = d1.subarchive()?;
            let state = ui_factory.create_state_from_archive(&obj, d2)?;
            self.set_object_data(id, state, color);
        }
        Ok(())
    }

    pub(super) fn make_states_for_new_objects(
        &mut self,
        topo: &SoundGraphTopology,
        ui_factory: &UiFactory,
    ) {
        let state_from_graph_object = |o: GraphObjectHandle| -> AnyObjectUiData {
            let state = ui_factory.create_default_state(&o);
            AnyObjectUiData {
                state,
                color: random_object_color(),
            }
        };

        for (i, spd) in topo.sound_processors() {
            self.data
                .entry(i.into())
                .or_insert_with(|| state_from_graph_object(spd.instance_arc().as_graph_object()));
        }
        for (i, nsd) in topo.number_sources() {
            if nsd.owner() != NumberSourceOwner::Nothing {
                continue;
            }
            self.data.entry(i.into()).or_insert_with(|| {
                state_from_graph_object(nsd.instance_arc().as_graph_object().unwrap())
            });
        }
    }
}
