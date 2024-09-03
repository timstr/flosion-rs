use std::{any::Any, collections::HashMap, sync::Arc};

use chive::ChiveIn;

use crate::{core::objecttype::ObjectType, ui_core::arguments::ParsedArguments};

use super::{expressiongraph::ExpressionGraph, expressionnode::ExpressionNodeId};

// TODO: remove Sync
pub trait ExpressionObject: Send + Sync {
    fn create(
        graph: &mut ExpressionGraph,
        args: &ParsedArguments,
    ) -> Result<AnyExpressionObjectHandle, ()>
    where
        Self: Sized;

    fn get_type() -> ObjectType
    where
        Self: Sized;

    fn get_dynamic_type(&self) -> ObjectType;

    fn get_id(&self) -> ExpressionNodeId;
    fn into_arc_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
    fn get_language_type_name(&self) -> &'static str;
    fn serialize(&self, chive_in: ChiveIn);
}

// TODO: this is used exclusively for looking up expression node types from handles
// and for downcasting type-erased handles. Rename it to something more suitable
pub trait ExpressionObjectHandle: Sized {
    // TODO: consider renaming this
    type ObjectType: ExpressionObject;

    fn from_graph_object(object: AnyExpressionObjectHandle) -> Option<Self>;

    fn object_type() -> ObjectType;
}

pub struct AnyExpressionObjectHandle {
    // TODO: just Rc? Or borrow?
    instance: Arc<dyn ExpressionObject>,
}

impl AnyExpressionObjectHandle {
    pub(crate) fn new(instance: Arc<dyn ExpressionObject>) -> Self {
        Self { instance }
    }

    pub(crate) fn id(&self) -> ExpressionNodeId {
        self.instance.get_id()
    }

    pub(crate) fn get_type(&self) -> ObjectType {
        self.instance.get_dynamic_type()
    }

    pub(crate) fn into_instance_arc(self) -> Arc<dyn ExpressionObject> {
        self.instance
    }
}

impl Clone for AnyExpressionObjectHandle {
    fn clone(&self) -> Self {
        Self {
            instance: Arc::clone(&self.instance),
        }
    }
}

struct ExpressionObjectData {
    create: Box<
        dyn Fn(&mut ExpressionGraph, &ParsedArguments) -> Result<AnyExpressionObjectHandle, ()>,
    >,
}

pub struct ExpressionObjectFactory {
    mapping: HashMap<&'static str, ExpressionObjectData>,
}

impl ExpressionObjectFactory {
    pub fn new_empty() -> ExpressionObjectFactory {
        ExpressionObjectFactory {
            mapping: HashMap::new(),
        }
    }

    pub fn register<T: ExpressionObject>(&mut self) {
        let create = |g: &mut ExpressionGraph,
                      args: &ParsedArguments|
         -> Result<AnyExpressionObjectHandle, ()> { T::create(g, args) };
        self.mapping.insert(
            T::get_type().name(),
            ExpressionObjectData {
                create: Box::new(create),
            },
        );
    }

    pub(crate) fn create(
        &self,
        object_type_str: &str,
        graph: &mut ExpressionGraph,
        args: &ParsedArguments,
    ) -> Result<AnyExpressionObjectHandle, ()> {
        match self.mapping.get(object_type_str) {
            Some(data) => (*data.create)(graph, args),
            None => panic!(
                "Tried to create an expression object of unrecognized type \"{}\"",
                object_type_str
            ),
        }
    }
}
