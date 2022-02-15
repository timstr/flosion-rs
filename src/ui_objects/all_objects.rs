use std::collections::HashMap;

use eframe::egui::Ui;
use futures::executor::block_on;

use crate::{
    core::{
        graphobject::{GraphObject, ObjectId, ObjectType, TypedGraphObject},
        numbersource::PureNumberSource,
        soundgraph::SoundGraph,
        soundprocessor::{DynamicSoundProcessor, StaticSoundProcessor},
    },
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{AnyObjectUi, ObjectUi},
    },
};

use super::{
    dac_ui::DacUi,
    functions_ui::{ConstantUi, UnitSineUi},
    wavegenerator_ui::WaveGeneratorUi,
};

struct ObjectData {
    ui: Box<dyn AnyObjectUi>,
    create: Box<dyn Fn(&mut SoundGraph)>,
}

pub struct AllObjects {
    mapping: HashMap<ObjectType, ObjectData>,
}

fn error_ui(ui: &mut Ui, object: &dyn GraphObject, object_type: ObjectType) {
    ui.label(format!(
        "[Unrecognized object type \"{}\" for type {}]",
        object_type.name(),
        object.get_language_type_name()
    ));
}

impl AllObjects {
    pub fn new() -> AllObjects {
        let mapping: HashMap<ObjectType, ObjectData> = HashMap::new();
        let mut all_uis = AllObjects { mapping };
        all_uis.register_static_sound_processor::<DacUi>();
        all_uis.register_dynamic_sound_processor::<WaveGeneratorUi>();
        all_uis.register_number_source::<ConstantUi>();
        all_uis.register_number_source::<UnitSineUi>();
        all_uis
    }

    pub fn register_dynamic_sound_processor<T: ObjectUi>(&mut self)
    where
        T::ObjectType: DynamicSoundProcessor,
    {
        let create = |g: &mut SoundGraph| {
            let _ = block_on(g.add_dynamic_sound_processor::<T::ObjectType>());
        };
        self.mapping.insert(
            T::ObjectType::TYPE,
            ObjectData {
                ui: Box::new(T::default()),
                create: Box::new(create),
            },
        );
    }

    pub fn register_static_sound_processor<T: ObjectUi>(&mut self)
    where
        T::ObjectType: StaticSoundProcessor,
    {
        let create = |g: &mut SoundGraph| {
            let _ = block_on(g.add_static_sound_processor::<T::ObjectType>());
        };
        self.mapping.insert(
            T::ObjectType::TYPE,
            ObjectData {
                ui: Box::new(T::default()),
                create: Box::new(create),
            },
        );
    }

    pub fn register_number_source<T: ObjectUi>(&mut self)
    where
        T::ObjectType: PureNumberSource,
    {
        let create = |g: &mut SoundGraph| {
            let _ = block_on(g.add_number_source::<T::ObjectType>());
        };
        self.mapping.insert(
            T::ObjectType::TYPE,
            ObjectData {
                ui: Box::new(T::default()),
                create: Box::new(create),
            },
        );
    }

    pub fn all_object_types(&self) -> Vec<ObjectType> {
        self.mapping.keys().cloned().collect()
    }

    pub fn ui(
        &self,
        id: ObjectId,
        object: &dyn GraphObject,
        object_type: ObjectType,
        graph_state: &mut GraphUIState,
        ui: &mut Ui,
    ) {
        match self.mapping.get(&object_type) {
            Some(data) => data.ui.apply(id, object, graph_state, ui),
            None => error_ui(ui, object, object_type),
        }
    }

    pub fn create(&self, object_type: ObjectType, graph: &mut SoundGraph) {
        match self.mapping.get(&object_type) {
            Some(data) => (*data.create)(graph),
            None => println!(
                "Warning: tried to create an object of unrecognized type \"{}\"",
                object_type.name()
            ),
        }
    }
}
