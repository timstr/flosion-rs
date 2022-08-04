use std::collections::HashMap;

use eframe::egui::Ui;

use crate::{
    core::{
        graphobject::{GraphObject, ObjectId, ObjectType, ObjectWrapper, WithObjectType},
        numbersource::PureNumberSource,
        serialization::Deserializer,
        soundgraph::SoundGraph,
        soundprocessor::SoundProcessor,
    },
    ui_core::{
        arguments::ParsedArguments,
        graph_ui_state::GraphUIState,
        object_ui::{AnyObjectUi, ObjectUi},
    },
};

use super::all_objects::all_objects;

enum ObjectInitialization<'a> {
    Args((&'a mut GraphUIState, &'a ParsedArguments)),
    Archive(Deserializer<'a>),
}

struct ObjectData {
    ui: Box<dyn AnyObjectUi>,

    // TODO: replace with e.g. Fn(Option<Deserializer>) -> &dyn AnyObjectUi
    // to remove the code duplication below
    create: Box<dyn Fn(&mut SoundGraph, &dyn AnyObjectUi, ObjectInitialization)>,
}

pub struct ObjectFactory {
    mapping: HashMap<ObjectType, ObjectData>,
}

fn error_ui(ui: &mut Ui, object: &dyn GraphObject, object_type: ObjectType) {
    ui.label(format!(
        "[Unrecognized object type \"{}\" for type {}]",
        object_type.name(),
        object.get_language_type_name()
    ));
}

impl ObjectFactory {
    pub fn new_empty() -> ObjectFactory {
        ObjectFactory {
            mapping: HashMap::new(),
        }
    }

    pub fn new() -> ObjectFactory {
        all_objects()
    }

    pub fn register_sound_processor<T: ObjectUi>(&mut self)
    where
        <T::WrapperType as ObjectWrapper>::Type: SoundProcessor,
    {
        let create = |g: &mut SoundGraph, _o: &dyn AnyObjectUi, _init: ObjectInitialization| {
            // TODO: either initialize from args or deserialize
            g.add_sound_processor::<<T::WrapperType as ObjectWrapper>::Type>();
        };
        self.mapping.insert(
            <T::WrapperType as ObjectWrapper>::Type::TYPE,
            ObjectData {
                ui: Box::new(T::default()),
                create: Box::new(create),
            },
        );
    }

    pub fn register_number_source<T: ObjectUi>(&mut self)
    where
        <T::WrapperType as ObjectWrapper>::Type: PureNumberSource,
    {
        let create = |g: &mut SoundGraph, _o: &dyn AnyObjectUi, _init: ObjectInitialization| {
            // TODO: either initialize from args or deserialize
            g.add_pure_number_source::<<T::WrapperType as ObjectWrapper>::Type>();
        };
        self.mapping.insert(
            <T::WrapperType as ObjectWrapper>::Type::TYPE,
            ObjectData {
                ui: Box::new(T::default()),
                create: Box::new(create),
            },
        );
    }

    pub fn all_object_types(&self) -> impl Iterator<Item = &ObjectType> {
        self.mapping.keys()
    }

    pub fn get_object_ui(&self, object_type: ObjectType) -> &dyn AnyObjectUi {
        &*self.mapping.get(&object_type).unwrap().ui
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
            Some(data) => {
                let state_rc = graph_state.get_object_state(id);
                let state_ref = state_rc.borrow();
                data.ui.apply(id, object, &*state_ref, graph_state, ui);
            }
            None => error_ui(ui, object, object_type),
        }
    }

    fn create_impl(
        &self,
        object_type: ObjectType,
        graph: &mut SoundGraph,
        init: ObjectInitialization,
    ) {
        match self.mapping.get(&object_type) {
            Some(data) => (*data.create)(graph, &*data.ui, init),
            None => println!(
                "Warning: tried to create an object of unrecognized type \"{}\"",
                object_type.name()
            ),
        }
    }

    pub fn create_from_args(
        &self,
        object_type: ObjectType,
        args: &ParsedArguments,
        graph: &mut SoundGraph,
        ui_state: &mut GraphUIState,
    ) {
        self.create_impl(
            object_type,
            graph,
            ObjectInitialization::Args((ui_state, args)),
        )
    }

    pub fn create_from_archive(
        &self,
        object_type: ObjectType,
        graph: &mut SoundGraph,
        deserializer: Deserializer,
    ) {
        self.create_impl(
            object_type,
            graph,
            ObjectInitialization::Archive(deserializer),
        )
    }
}
