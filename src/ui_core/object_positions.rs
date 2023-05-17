use std::collections::{HashMap, HashSet};

use eframe::egui;

use crate::core::{
    graphobject::{GraphId, ObjectId},
    graphserialization::{
        deserialize_object_id, serialize_object_id, ForwardGraphIdMap, ReverseGraphIdMap,
    },
    serialization::{Deserializer, Serializer},
};

pub struct LayoutState {
    pub rect: egui::Rect,
}

impl LayoutState {
    pub fn center(&self) -> egui::Pos2 {
        self.rect.center()
    }
}

pub struct ObjectPositions {
    objects: HashMap<ObjectId, LayoutState>,
}

impl ObjectPositions {
    pub(super) fn new() -> ObjectPositions {
        ObjectPositions {
            objects: HashMap::new(),
        }
    }

    pub(super) fn retain(&mut self, ids: &HashSet<GraphId>) {
        self.objects.retain(|i, _| ids.contains(&(*i).into()));
    }

    pub(super) fn objects(&self) -> &HashMap<ObjectId, LayoutState> {
        &self.objects
    }

    pub(super) fn objects_mut(&mut self) -> &mut HashMap<ObjectId, LayoutState> {
        &mut self.objects
    }

    pub fn track_object_location(&mut self, id: ObjectId, rect: egui::Rect) {
        self.objects.insert(id, LayoutState { rect });
    }

    pub fn get_object_location(&self, id: ObjectId) -> Option<&LayoutState> {
        self.objects.get(&id)
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
                // TODO: make a better default, don't assume the object ui will overwrite this
                rect: egui::Rect::NAN,
            });
            layout.rect.set_left(left);
            layout.rect.set_top(top);
        }
        Ok(())
    }

    pub(crate) fn create_state_for(&mut self, object_id: ObjectId) {
        // TODO: allow the position to be passed in
        self.objects
            .entry(object_id)
            .or_insert_with(|| LayoutState {
                rect: egui::Rect::from_center_size(egui::pos2(400.0, 400.0), egui::Vec2::ZERO),
            });
    }
}
