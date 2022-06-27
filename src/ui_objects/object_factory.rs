use std::collections::HashMap;

use eframe::egui::Ui;
use futures::executor::block_on;

use crate::{
    core::{
        graphobject::{GraphObject, ObjectId, ObjectType, ObjectWrapper, WithObjectType},
        numbersource::PureNumberSource,
        soundgraph::SoundGraph,
        soundprocessor::{DynamicSoundProcessor, StaticSoundProcessor},
    },
    ui_core::{
        arguments::ParsedArguments,
        graph_ui_state::GraphUIState,
        object_ui::{AnyObjectUi, ObjectUi},
    },
};

use super::all_objects::all_objects;

struct ObjectData {
    ui: Box<dyn AnyObjectUi>,
    create: Box<dyn Fn(&mut SoundGraph, &mut GraphUIState, &dyn AnyObjectUi, &ParsedArguments)>,
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

    pub fn register_dynamic_sound_processor<T: ObjectUi>(&mut self)
    where
        <T::WrapperType as ObjectWrapper>::Type: DynamicSoundProcessor,
    {
        let create = |g: &mut SoundGraph,
                      s: &mut GraphUIState,
                      o: &dyn AnyObjectUi,
                      args: &ParsedArguments| {
            let h = block_on(
                g.add_dynamic_sound_processor::<<T::WrapperType as ObjectWrapper>::Type>(),
            );
            let sp: &dyn GraphObject = h.wrapper();
            o.init_object(sp, args);
            s.set_object_state(h.id().into(), o.make_state(args));
        };
        self.mapping.insert(
            <T::WrapperType as ObjectWrapper>::Type::TYPE,
            ObjectData {
                ui: Box::new(T::default()),
                create: Box::new(create),
            },
        );
    }

    pub fn register_static_sound_processor<T: ObjectUi>(&mut self)
    where
        <T::WrapperType as ObjectWrapper>::Type: StaticSoundProcessor,
    {
        let create = |g: &mut SoundGraph,
                      s: &mut GraphUIState,
                      o: &dyn AnyObjectUi,
                      args: &ParsedArguments| {
            let h =
                block_on(g.add_static_sound_processor::<<T::WrapperType as ObjectWrapper>::Type>());
            let sp: &dyn GraphObject = h.wrapper();
            o.init_object(sp, args);
            s.set_object_state(h.id().into(), o.make_state(args));
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
        let create = |g: &mut SoundGraph,
                      s: &mut GraphUIState,
                      o: &dyn AnyObjectUi,
                      args: &ParsedArguments| {
            let h = block_on(g.add_number_source::<<T::WrapperType as ObjectWrapper>::Type>());
            let ns: &dyn GraphObject = h.instance();
            o.init_object(ns, args);
            s.set_object_state(h.id().into(), o.make_state(args));
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

    pub fn create(
        &self,
        object_type: ObjectType,
        args: &ParsedArguments,
        graph: &mut SoundGraph,
        ui_state: &mut GraphUIState,
    ) {
        match self.mapping.get(&object_type) {
            Some(data) => (*data.create)(graph, ui_state, &*data.ui, args),
            None => println!(
                "Warning: tried to create an object of unrecognized type \"{}\"",
                object_type.name()
            ),
        }
    }
}
