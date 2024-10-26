use std::{any::Any, collections::HashMap};

use crate::{core::objecttype::ObjectType, ui_core::arguments::ParsedArguments};

use super::expressionnode::{AnyExpressionNode, ExpressionNodeId};

pub trait ExpressionObject {
    fn create(args: &ParsedArguments) -> Self
    where
        Self: Sized;

    fn id(&self) -> ExpressionNodeId;

    fn get_type() -> ObjectType
    where
        Self: Sized;

    fn get_dynamic_type(&self) -> ObjectType;

    fn as_expression_node(&self) -> Option<&dyn AnyExpressionNode>;
    fn into_boxed_expression_node(self: Box<Self>) -> Option<Box<dyn AnyExpressionNode>>;

    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;

    // TODO: remove
    fn get_language_type_name(&self) -> &'static str;
}

struct ExpressionObjectCreator {
    create: Box<dyn Fn(&ParsedArguments) -> Box<dyn ExpressionObject>>,
}

pub struct ExpressionObjectFactory {
    mapping: HashMap<&'static str, ExpressionObjectCreator>,
}

impl ExpressionObjectFactory {
    pub fn new_empty() -> ExpressionObjectFactory {
        ExpressionObjectFactory {
            mapping: HashMap::new(),
        }
    }

    pub fn register<T: 'static + ExpressionObject>(&mut self) {
        self.mapping.insert(
            T::get_type().name(),
            ExpressionObjectCreator {
                create: Box::new(|args| Box::new(T::create(args))),
            },
        );
    }

    pub(crate) fn create(
        &self,
        object_type_str: &str,
        args: &ParsedArguments,
    ) -> Box<dyn ExpressionObject> {
        match self.mapping.get(object_type_str) {
            Some(data) => (*data.create)(args),
            None => panic!(
                "Tried to create an expression object of unrecognized type \"{}\"",
                object_type_str
            ),
        }
    }
}
