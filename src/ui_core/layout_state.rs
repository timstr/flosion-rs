use std::collections::{HashMap, HashSet};

use eframe::egui;

use crate::core::{
    graphobject::{GraphId, ObjectId},
    graphserialization::{
        deserialize_object_id, serialize_object_id, ForwardGraphIdMap, ReverseGraphIdMap,
    },
    numberinput::NumberInputId,
    numbersource::NumberSourceId,
    serialization::{Deserializer, Serializer},
    soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
    uniqueid::UniqueId,
};

use super::object_ui::PegDirection;

pub struct LayoutState {
    pub rect: egui::Rect,
    pub layer: egui::LayerId,
}

impl LayoutState {
    pub fn center(&self) -> egui::Pos2 {
        self.rect.center()
    }
}

pub struct PegLayoutState {
    pub direction: PegDirection,
    pub layout: LayoutState,
}

pub struct GraphLayout {
    sound_inputs: HashMap<SoundInputId, PegLayoutState>,
    sound_outputs: HashMap<SoundProcessorId, PegLayoutState>,
    number_inputs: HashMap<NumberInputId, PegLayoutState>,
    number_outputs: HashMap<NumberSourceId, PegLayoutState>,
    objects: HashMap<ObjectId, LayoutState>,
}

impl GraphLayout {
    pub(super) fn new() -> GraphLayout {
        GraphLayout {
            sound_inputs: HashMap::new(),
            sound_outputs: HashMap::new(),
            number_inputs: HashMap::new(),
            number_outputs: HashMap::new(),
            objects: HashMap::new(),
        }
    }

    pub(super) fn reset_pegs(&mut self) {
        self.sound_inputs.clear();
        self.sound_outputs.clear();
        self.number_inputs.clear();
        self.number_outputs.clear();
    }

    pub(super) fn retain(&mut self, ids: &HashSet<GraphId>) {
        self.objects.retain(|i, _| ids.contains(&(*i).into()));
        self.sound_inputs.retain(|i, _| ids.contains(&(*i).into()));
        self.sound_outputs.retain(|i, _| ids.contains(&(*i).into()));
        self.number_inputs.retain(|i, _| ids.contains(&(*i).into()));
        self.number_outputs
            .retain(|i, _| ids.contains(&(*i).into()));
    }

    pub fn track_peg(
        &mut self,
        id: GraphId,
        rect: egui::Rect,
        layer: egui::LayerId,
        direction: PegDirection,
    ) {
        let state = PegLayoutState {
            direction,
            layout: LayoutState { rect, layer },
        };
        match id {
            GraphId::NumberInput(id) => self.number_inputs.insert(id, state),
            GraphId::NumberSource(id) => self.number_outputs.insert(id, state),
            GraphId::SoundInput(id) => self.sound_inputs.insert(id, state),
            GraphId::SoundProcessor(id) => self.sound_outputs.insert(id, state),
        };
    }

    pub(super) fn objects(&self) -> &HashMap<ObjectId, LayoutState> {
        &self.objects
    }

    pub(super) fn objects_mut(&mut self) -> &mut HashMap<ObjectId, LayoutState> {
        &mut self.objects
    }

    pub fn track_object_location(&mut self, id: ObjectId, rect: egui::Rect, layer: egui::LayerId) {
        self.objects.insert(id, LayoutState { rect, layer });
    }

    pub fn get_object_location(&self, id: ObjectId) -> Option<&LayoutState> {
        self.objects.get(&id)
    }

    pub(super) fn sound_inputs(&self) -> &HashMap<SoundInputId, PegLayoutState> {
        &self.sound_inputs
    }

    pub(super) fn sound_outputs(&self) -> &HashMap<SoundProcessorId, PegLayoutState> {
        &self.sound_outputs
    }

    pub(super) fn number_inputs(&self) -> &HashMap<NumberInputId, PegLayoutState> {
        &self.number_inputs
    }

    pub(super) fn number_outputs(&self) -> &HashMap<NumberSourceId, PegLayoutState> {
        &self.number_outputs
    }

    pub(super) fn find_peg_near(&self, position: egui::Pos2, ui: &egui::Ui) -> Option<GraphId> {
        let rad = ui.input(|i| i.aim_radius());
        let top_layer = match ui.memory(|m| m.layer_id_at(position, rad)) {
            Some(a) => a,
            None => return None,
        };
        fn find<T: UniqueId>(
            peg_states: &HashMap<T, PegLayoutState>,
            layer: egui::LayerId,
            position: egui::Pos2,
        ) -> Option<T> {
            for (id, st) in peg_states {
                if st.layout.layer != layer {
                    continue;
                }
                if st.layout.rect.contains(position) {
                    return Some(*id);
                }
            }
            None
        }

        if let Some(id) = find(&self.number_inputs, top_layer, position) {
            return Some(id.into());
        }
        if let Some(id) = find(&self.number_outputs, top_layer, position) {
            return Some(id.into());
        }
        if let Some(id) = find(&self.sound_inputs, top_layer, position) {
            return Some(id.into());
        }
        if let Some(id) = find(&self.sound_outputs, top_layer, position) {
            return Some(id.into());
        }
        None
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
        for (id, layout) in &self.objects {
            if !is_selected(*id) {
                continue;
            }
            serialize_object_id(*id, &mut s1, idmap);
            s1.f32(layout.rect.left());
            s1.f32(layout.rect.top());
        }
    }

    pub(super) fn deserialize(
        &mut self,
        deserializer: &mut Deserializer,
        idmap: &ReverseGraphIdMap,
    ) -> Result<(), ()> {
        let mut d1 = deserializer.subarchive()?;
        while !d1.is_empty() {
            let id: ObjectId = deserialize_object_id(&mut d1, idmap)?;
            let left = d1.f32()?;
            let top = d1.f32()?;
            let layout = self.objects.entry(id).or_insert(LayoutState {
                rect: egui::Rect::NAN,
                layer: egui::LayerId::debug(),
            });
            layout.rect.set_left(left);
            layout.rect.set_top(top);
        }
        Ok(())
    }
}
