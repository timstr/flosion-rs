use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::core::{
    expression::expressiongraph::{ExpressionGraph, ExpressionGraphParameterId},
    uniqueid::UniqueId,
};

use super::{
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
pub(crate) struct ExpressionParameterMapping {
    mapping: HashMap<ExpressionGraphParameterId, SoundExpressionArgumentId>,
}

impl ExpressionParameterMapping {
    pub(crate) fn new() -> ExpressionParameterMapping {
        ExpressionParameterMapping {
            mapping: HashMap::new(),
        }
    }

    pub(crate) fn argument_from_parameter(
        &self,
        id: ExpressionGraphParameterId,
    ) -> Option<SoundExpressionArgumentId> {
        self.mapping.get(&id).cloned()
    }

    pub(crate) fn parameter_from_argument(
        &self,
        id: SoundExpressionArgumentId,
    ) -> Option<ExpressionGraphParameterId> {
        for (giid, nsid) in &self.mapping {
            if *nsid == id {
                return Some(*giid);
            }
        }
        None
    }

    // NOTE: passing ExpressionGraph separately here might seem a bit awkward from the perspective of the
    // SoundExpressionData that owns this and the expression graph, but it makes the two separable.
    // This is useful for making LexicalLayout more reusable accross different types of expressions
    pub(crate) fn add_argument(
        &mut self,
        argument_id: SoundExpressionArgumentId,
        expr_graph: &mut ExpressionGraph,
    ) -> ExpressionGraphParameterId {
        debug_assert!(self.check_invariants(expr_graph));
        if let Some(giid) = self.parameter_from_argument(argument_id) {
            return giid;
        }
        let giid = expr_graph.add_parameter();
        let prev = self.mapping.insert(giid, argument_id);
        debug_assert_eq!(prev, None);
        debug_assert!(self.check_invariants(expr_graph));
        giid
    }

    pub(crate) fn remove_argument(
        &mut self,
        argument_id: SoundExpressionArgumentId,
        expr_graph: &mut ExpressionGraph,
    ) {
        debug_assert!(self.check_invariants(expr_graph));
        let giid = self.parameter_from_argument(argument_id).unwrap();
        expr_graph.remove_parameter(giid).unwrap();
        let prev = self.mapping.remove(&giid);
        debug_assert!(prev.is_some());
        debug_assert!(self.check_invariants(expr_graph));
    }

    fn check_invariants(&self, graph: &ExpressionGraph) -> bool {
        let mapped_params: HashSet<ExpressionGraphParameterId> =
            self.mapping.keys().cloned().collect();
        let actual_params: HashSet<ExpressionGraphParameterId> =
            graph.parameters().iter().cloned().collect();
        if mapped_params != actual_params {
            println!("Expression parameters were modified without updating parameter mapping");
            false
        } else {
            true
        }
    }

    pub(crate) fn items(&self) -> &HashMap<ExpressionGraphParameterId, SoundExpressionArgumentId> {
        &self.mapping
    }
}

#[derive(Clone)]
pub struct SoundExpressionScope {
    processor_state_available: bool,
    available_local_arguments: Vec<SoundExpressionArgumentId>,
}

impl SoundExpressionScope {
    pub fn without_processor_state() -> SoundExpressionScope {
        SoundExpressionScope {
            processor_state_available: false,
            available_local_arguments: Vec::new(),
        }
    }

    pub fn with_processor_state() -> SoundExpressionScope {
        SoundExpressionScope {
            processor_state_available: true,
            available_local_arguments: Vec::new(),
        }
    }

    pub fn add_local(mut self, id: SoundExpressionArgumentId) -> SoundExpressionScope {
        self.available_local_arguments.push(id);
        self
    }

    pub(crate) fn processor_state_available(&self) -> bool {
        self.processor_state_available
    }

    pub(crate) fn available_local_arguments(&self) -> &[SoundExpressionArgumentId] {
        &self.available_local_arguments
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
