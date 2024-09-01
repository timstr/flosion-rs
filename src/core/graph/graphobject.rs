use std::{any::Any, sync::Arc};

use chive::ChiveIn;

use crate::ui_core::arguments::ParsedArguments;

use super::graph::Graph;

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct ObjectType {
    name: &'static str,
}

impl ObjectType {
    pub const fn new(name: &'static str) -> ObjectType {
        ObjectType { name }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }
}

pub trait GraphObject<G: Graph>: 'static + Send + Sync {
    fn create(graph: &mut G, args: ParsedArguments) -> Result<GraphObjectHandle<G>, ()>
    where
        Self: Sized;

    fn get_type() -> ObjectType
    where
        Self: Sized;

    fn get_dynamic_type(&self) -> ObjectType;

    fn get_id(&self) -> G::ObjectId;
    fn into_arc_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
    fn get_language_type_name(&self) -> &'static str;
    fn serialize(&self, chive_in: ChiveIn);
}

pub struct GraphObjectHandle<G: Graph> {
    instance: Arc<dyn GraphObject<G>>,
}

impl<G: Graph> GraphObjectHandle<G> {
    pub(crate) fn new(instance: Arc<dyn GraphObject<G>>) -> Self {
        Self { instance }
    }

    pub(crate) fn id(&self) -> G::ObjectId {
        self.instance.get_id()
    }

    pub(crate) fn get_type(&self) -> ObjectType {
        self.instance.get_dynamic_type()
    }

    pub(crate) fn into_instance_arc(self) -> Arc<dyn GraphObject<G>> {
        self.instance
    }
}

impl<G: Graph> Clone for GraphObjectHandle<G> {
    fn clone(&self) -> Self {
        Self {
            instance: Arc::clone(&self.instance),
        }
    }
}

// Used by ObjectUi to specify the handle type and inner object type
// that a UI works with
pub trait ObjectHandle<G: Graph>: Sized {
    // TODO: consider renaming this
    type ObjectType: GraphObject<G>;

    fn from_graph_object(object: GraphObjectHandle<G>) -> Option<Self>;

    fn object_type() -> ObjectType;
}

pub trait WithObjectType: 'static {
    const TYPE: ObjectType;
}
