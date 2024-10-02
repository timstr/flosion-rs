use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::core::{
    expression::expressiongraph::{ExpressionGraph, ExpressionGraphParameterId},
    uniqueid::UniqueId,
};

use super::{
    expression::{ProcessorExpression, ProcessorExpressionId},
    expressionargument::{
        SoundExpressionArgument, SoundExpressionArgumentId, SoundExpressionArgumentOwner,
    },
    soundinput::{InputOptions, SoundInputId},
    soundprocessor::{SoundProcessor, SoundProcessorId},
};

pub struct SoundInputBranchTag;

/// Unique integer identifier for each of the branches stemming
/// from a given sound input.
pub type SoundInputBranchId = UniqueId<SoundInputBranchTag>;

#[derive(Clone)]
pub(crate) struct SoundInputData {
    id: SoundInputId,
    options: InputOptions,
    branches: Vec<SoundInputBranchId>,
    target: Option<SoundProcessorId>,
    owner: SoundProcessorId,
    arguments: Vec<SoundExpressionArgumentId>,
    time_argument: Option<SoundExpressionArgumentId>,
}

impl SoundInputData {
    pub(super) fn new(
        id: SoundInputId,
        options: InputOptions,
        branches: Vec<SoundInputBranchId>,
        owner: SoundProcessorId,
    ) -> SoundInputData {
        SoundInputData {
            id,
            options,
            branches,
            target: None,
            owner,
            arguments: Vec::new(),
            time_argument: None,
        }
    }

    pub(crate) fn id(&self) -> SoundInputId {
        self.id
    }

    pub(crate) fn options(&self) -> InputOptions {
        self.options
    }

    pub(crate) fn branches(&self) -> &[SoundInputBranchId] {
        &self.branches
    }

    pub(super) fn branches_mut(&mut self) -> &mut Vec<SoundInputBranchId> {
        &mut self.branches
    }

    pub(crate) fn target(&self) -> Option<SoundProcessorId> {
        self.target
    }

    pub(super) fn set_target(&mut self, target: Option<SoundProcessorId>) {
        self.target = target;
    }

    pub(crate) fn owner(&self) -> SoundProcessorId {
        self.owner
    }

    pub(crate) fn expression_arguments(&self) -> &[SoundExpressionArgumentId] {
        &self.arguments
    }

    pub(crate) fn arguments(&self) -> &[SoundExpressionArgumentId] {
        &self.arguments
    }

    pub(crate) fn arguments_mut(&mut self) -> &mut Vec<SoundExpressionArgumentId> {
        &mut self.arguments
    }

    pub(crate) fn time_argument(&self) -> SoundExpressionArgumentId {
        self.time_argument.unwrap()
    }

    pub(super) fn set_time_argument(&mut self, arg_id: SoundExpressionArgumentId) {
        debug_assert!(self.time_argument.is_none());
        self.time_argument = Some(arg_id);
    }
}

#[derive(Clone)]
pub(crate) struct SoundProcessorData {
    id: SoundProcessorId,
    processor: Option<Rc<dyn SoundProcessor>>,
    sound_inputs: Vec<SoundInputId>,
    arguments: Vec<SoundExpressionArgumentId>,
}

impl SoundProcessorData {
    pub(super) fn new_empty(id: SoundProcessorId) -> SoundProcessorData {
        SoundProcessorData {
            id,
            processor: None,
            sound_inputs: Vec::new(),
            arguments: Vec::new(),
        }
    }

    pub(crate) fn set_processor(&mut self, processor: Rc<dyn SoundProcessor>) {
        assert!(self.processor.is_none());
        assert!(processor.id() == self.id());
        self.processor = Some(processor);
    }

    pub(crate) fn id(&self) -> SoundProcessorId {
        self.id
    }

    pub(crate) fn sound_inputs(&self) -> &[SoundInputId] {
        &self.sound_inputs
    }

    pub(super) fn sound_inputs_mut(&mut self) -> &mut Vec<SoundInputId> {
        &mut self.sound_inputs
    }

    pub(crate) fn expression_arguments(&self) -> &[SoundExpressionArgumentId] {
        &self.arguments
    }

    pub(crate) fn foreach_expression<F: FnMut(&ProcessorExpression)>(&self, f: F) {
        self.processor
            .as_ref()
            .unwrap()
            .visit_expressions(Box::new(f));
    }

    pub(crate) fn with_expression<F: FnMut(&ProcessorExpression)>(
        &self,
        id: ProcessorExpressionId,
        mut f: F,
    ) {
        self.processor
            .as_ref()
            .unwrap()
            .visit_expressions(Box::new(|expr| {
                if expr.id() == id {
                    f(expr);
                }
            }));
    }

    pub(crate) fn with_expression_mut<F: FnMut(&mut ProcessorExpression)>(
        &self,
        id: ProcessorExpressionId,
        mut f: F,
    ) {
        self.processor
            .as_ref()
            .unwrap()
            .visit_expressions_mut(Box::new(|expr| {
                if expr.id() == id {
                    f(expr);
                }
            }));
    }

    pub(super) fn arguments(&self) -> &Vec<SoundExpressionArgumentId> {
        &self.arguments
    }

    pub(super) fn arguments_mut(&mut self) -> &mut Vec<SoundExpressionArgumentId> {
        &mut self.arguments
    }

    pub(crate) fn instance(&self) -> &dyn SoundProcessor {
        self.processor.as_deref().unwrap()
    }

    pub(crate) fn instance_rc(&self) -> Rc<dyn SoundProcessor> {
        Rc::clone(self.processor.as_ref().unwrap())
    }

    pub(crate) fn friendly_name(&self) -> String {
        format!(
            "{}#{}",
            self.instance_rc().as_graph_object().get_type().name(),
            self.id.value()
        )
    }
}

#[derive(Clone)]
pub(crate) struct SoundExpressionArgumentData {
    id: SoundExpressionArgumentId,
    instance: Rc<dyn SoundExpressionArgument>,
    owner: SoundExpressionArgumentOwner,
}

impl SoundExpressionArgumentData {
    pub(super) fn new(
        id: SoundExpressionArgumentId,
        instance: Rc<dyn SoundExpressionArgument>,
        owner: SoundExpressionArgumentOwner,
    ) -> Self {
        Self {
            id,
            instance,
            owner,
        }
    }

    pub(crate) fn id(&self) -> SoundExpressionArgumentId {
        self.id
    }

    pub(crate) fn instance(&self) -> &dyn SoundExpressionArgument {
        &*self.instance
    }

    pub(crate) fn owner(&self) -> SoundExpressionArgumentOwner {
        self.owner
    }
}
