use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use eframe::egui::Ui;

use crate::{
    core::{
        graphobject::{GraphObject, ObjectId, ObjectType, TypedGraphObject, WithObjectType},
        numbersource::PureNumberSource,
        serialization::Deserializer,
        serialization::Serializable,
        soundgraph::SoundGraph,
        soundprocessor::SoundProcessor,
    },
    ui_core::{
        arguments::ParsedArguments,
        graph_ui_state::GraphUIState,
        object_ui::{AnyObjectUi, ObjectUi},
    },
};

use crate::ui_objects::all_objects::all_objects;

enum ObjectInitialization<'a> {
    Args(&'a ParsedArguments),
    Archive {
        object_state: Deserializer<'a>,
        ui_state: Deserializer<'a>,
    },
}

struct ObjectData {
    ui: Box<dyn AnyObjectUi>,

    create: Box<
        dyn Fn(
            &mut SoundGraph,
            &mut GraphUIState,
            &dyn AnyObjectUi,
            ObjectInitialization,
        ) -> ObjectId,
    >,
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
        <T::WrapperType as TypedGraphObject>::Type: SoundProcessor,
    {
        let create = |g: &mut SoundGraph,
                      u: &mut GraphUIState,
                      o: &dyn AnyObjectUi,
                      init: ObjectInitialization|
         -> ObjectId {
            let h = g.add_sound_processor::<<T::WrapperType as TypedGraphObject>::Type>();
            match init {
                ObjectInitialization::Args(a) => {
                    o.init_object_from_args(&h, a);
                    u.set_object_state(h.id().into(), o.make_ui_state(a));
                }
                ObjectInitialization::Archive {
                    mut object_state,
                    mut ui_state,
                } => {
                    o.init_object_from_archive(&h, &mut object_state);
                    let state = T::StateType::deserialize(&mut ui_state).unwrap();
                    let state = Rc::new(RefCell::new(state));
                    u.set_object_state(h.id().into(), state);
                }
            }
            ObjectId::Sound(h.id())
        };
        self.mapping.insert(
            <T::WrapperType as TypedGraphObject>::Type::TYPE,
            ObjectData {
                ui: Box::new(T::default()),
                create: Box::new(create),
            },
        );
    }

    pub fn register_number_source<T: ObjectUi>(&mut self)
    where
        <T::WrapperType as TypedGraphObject>::Type: PureNumberSource,
    {
        let create = |g: &mut SoundGraph,
                      u: &mut GraphUIState,
                      o: &dyn AnyObjectUi,
                      init: ObjectInitialization| {
            let h = g.add_pure_number_source::<<T::WrapperType as TypedGraphObject>::Type>();
            match init {
                ObjectInitialization::Args(a) => {
                    o.init_object_from_args(&h, a);
                    u.set_object_state(h.id().into(), o.make_ui_state(a));
                }
                ObjectInitialization::Archive {
                    mut object_state,
                    mut ui_state,
                } => {
                    o.init_object_from_archive(&h, &mut object_state);
                    let state = T::StateType::deserialize(&mut ui_state).unwrap();
                    let state = Rc::new(RefCell::new(state));
                    u.set_object_state(h.id().into(), state);
                }
            }
            ObjectId::Number(h.id())
        };
        self.mapping.insert(
            <T::WrapperType as TypedGraphObject>::Type::TYPE,
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
        ui_state: &mut GraphUIState,
        init: ObjectInitialization,
    ) -> ObjectId {
        match self.mapping.get(&object_type) {
            Some(data) => (*data.create)(graph, ui_state, &*data.ui, init),
            None => panic!(
                "Tried to create an object of unrecognized type \"{}\"",
                object_type.name()
            ),
        }
    }

    pub fn create_from_args(
        &self,
        object_type: ObjectType,
        graph: &mut SoundGraph,
        ui_state: &mut GraphUIState,
        args: &ParsedArguments,
    ) -> ObjectId {
        self.create_impl(
            object_type,
            graph,
            ui_state,
            ObjectInitialization::Args(args),
        )
    }

    pub fn create_from_archive(
        &self,
        object_type: ObjectType,
        graph: &mut SoundGraph,
        ui_state: &mut GraphUIState,
        object_deserializer: Deserializer,
        ui_deserializer: Deserializer,
    ) -> ObjectId {
        self.create_impl(
            object_type,
            graph,
            ui_state,
            ObjectInitialization::Archive {
                object_state: object_deserializer,
                ui_state: ui_deserializer,
            },
        )
    }
}
