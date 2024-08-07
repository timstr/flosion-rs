use std::{
    collections::{HashMap, HashSet},
    hash::Hasher,
    sync::Arc,
};

use hashrevise::{Revisable, RevisionHash, RevisionHasher};

use crate::core::{
    expression::{
        expressiongraph::{ExpressionGraph, ExpressionGraphParameterId},
        expressiongraphtopology::ExpressionGraphTopology,
    },
    uniqueid::UniqueId,
};

use super::{
    expression::SoundExpressionId,
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
    time_argument: SoundExpressionArgumentId,
}

impl SoundInputData {
    pub(super) fn new(
        id: SoundInputId,
        options: InputOptions,
        branches: Vec<SoundInputBranchId>,
        owner: SoundProcessorId,
        time_argument: SoundExpressionArgumentId,
    ) -> SoundInputData {
        SoundInputData {
            id,
            options,
            branches,
            target: None,
            owner,
            arguments: Vec::new(),
            time_argument,
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

    pub(crate) fn expression_arguments(&self) -> &Vec<SoundExpressionArgumentId> {
        &self.arguments
    }

    pub(crate) fn arguments(&self) -> &Vec<SoundExpressionArgumentId> {
        &self.arguments
    }

    pub(crate) fn arguments_mut(&mut self) -> &mut Vec<SoundExpressionArgumentId> {
        &mut self.arguments
    }

    pub(crate) fn time_argument(&self) -> SoundExpressionArgumentId {
        self.time_argument
    }
}

impl Revisable for SoundInputData {
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = RevisionHasher::new();
        hasher.write_usize(self.id.value());
        hasher.write_u8(match &self.options {
            InputOptions::Synchronous => 0x1,
            InputOptions::NonSynchronous => 0x2,
        });
        hasher.write_revisable(&self.branches);
        hasher.write_usize(match &self.target {
            Some(id) => id.value(),
            None => usize::MAX,
        });
        hasher.write_usize(self.owner.value());
        hasher.write_revisable(&self.arguments);
        hasher.into_revision()
    }
}

#[derive(Clone)]
pub(crate) struct SoundProcessorData {
    id: SoundProcessorId,
    processor: Option<Arc<dyn SoundProcessor>>,
    sound_inputs: Vec<SoundInputId>,
    arguments: Vec<SoundExpressionArgumentId>,
    expressions: Vec<SoundExpressionId>,
}

impl SoundProcessorData {
    pub(crate) fn new(processor: Arc<dyn SoundProcessor>) -> SoundProcessorData {
        SoundProcessorData {
            id: processor.id(),
            processor: Some(processor),
            sound_inputs: Vec::new(),
            arguments: Vec::new(),
            expressions: Vec::new(),
        }
    }

    pub(crate) fn new_empty(id: SoundProcessorId) -> SoundProcessorData {
        SoundProcessorData {
            id,
            processor: None,
            sound_inputs: Vec::new(),
            arguments: Vec::new(),
            expressions: Vec::new(),
        }
    }

    pub(crate) fn set_processor(&mut self, processor: Arc<dyn SoundProcessor>) {
        assert!(self.processor.is_none());
        assert!(processor.id() == self.id());
        self.processor = Some(processor);
    }

    pub(crate) fn id(&self) -> SoundProcessorId {
        self.id
    }

    pub(crate) fn sound_inputs(&self) -> &Vec<SoundInputId> {
        &self.sound_inputs
    }

    pub(super) fn sound_inputs_mut(&mut self) -> &mut Vec<SoundInputId> {
        &mut self.sound_inputs
    }

    pub(crate) fn expression_arguments(&self) -> &Vec<SoundExpressionArgumentId> {
        &self.arguments
    }

    pub(super) fn arguments_mut(&mut self) -> &mut Vec<SoundExpressionArgumentId> {
        &mut self.arguments
    }

    pub(crate) fn expressions(&self) -> &Vec<SoundExpressionId> {
        &self.expressions
    }

    pub(super) fn expressions_mut(&mut self) -> &mut Vec<SoundExpressionId> {
        &mut self.expressions
    }

    pub(crate) fn instance(&self) -> &dyn SoundProcessor {
        self.processor.as_deref().unwrap()
    }

    pub(crate) fn instance_arc(&self) -> Arc<dyn SoundProcessor> {
        Arc::clone(self.processor.as_ref().unwrap())
    }

    pub(crate) fn friendly_name(&self) -> String {
        format!(
            "{}#{}",
            self.instance_arc().as_graph_object().get_type().name(),
            self.id.value()
        )
    }
}

impl Revisable for SoundProcessorData {
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = RevisionHasher::new();
        hasher.write_usize(self.id.value());
        hasher.write_u8(if self.instance().is_static() { 1 } else { 2 });
        // Do not hash processor instance
        hasher.write_revisable(&self.sound_inputs);
        hasher.write_revisable(&self.arguments);
        hasher.write_revisable(&self.expressions);
        hasher.into_revision()
    }
}

#[derive(Clone)]
pub(crate) struct ExpressionParameterMapping {
    mapping: HashMap<ExpressionGraphParameterId, SoundExpressionArgumentId>,
}

impl ExpressionParameterMapping {
    fn new() -> ExpressionParameterMapping {
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
        debug_assert!(self.check_invariants(expr_graph.topology()));
        if let Some(giid) = self.parameter_from_argument(argument_id) {
            return giid;
        }
        let giid = expr_graph.add_parameter();
        let prev = self.mapping.insert(giid, argument_id);
        debug_assert_eq!(prev, None);
        debug_assert!(self.check_invariants(expr_graph.topology()));
        giid
    }

    pub(crate) fn remove_argument(
        &mut self,
        argument_id: SoundExpressionArgumentId,
        expr_graph: &mut ExpressionGraph,
    ) {
        debug_assert!(self.check_invariants(expr_graph.topology()));
        let giid = self.parameter_from_argument(argument_id).unwrap();
        expr_graph.remove_parameter(giid).unwrap();
        let prev = self.mapping.remove(&giid);
        debug_assert!(prev.is_some());
        debug_assert!(self.check_invariants(expr_graph.topology()));
    }

    fn check_invariants(&self, topology: &ExpressionGraphTopology) -> bool {
        let mapped_params: HashSet<ExpressionGraphParameterId> =
            self.mapping.keys().cloned().collect();
        let actual_params: HashSet<ExpressionGraphParameterId> =
            topology.parameters().iter().cloned().collect();
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

impl Revisable for ExpressionParameterMapping {
    fn get_revision(&self) -> RevisionHash {
        self.mapping.get_revision()
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

impl Revisable for SoundExpressionScope {
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = RevisionHasher::new();
        hasher.write_u8(if self.processor_state_available { 1 } else { 0 });
        hasher.write_revisable(&self.available_local_arguments);
        hasher.into_revision()
    }
}

#[derive(Clone)]
pub struct SoundExpressionData {
    id: SoundExpressionId,
    target_mapping: ExpressionParameterMapping,
    expression_graph: ExpressionGraph,
    owner: SoundProcessorId,
    scope: SoundExpressionScope,
}

impl SoundExpressionData {
    pub(crate) fn new(
        id: SoundExpressionId,
        owner: SoundProcessorId,
        default_value: f32,
        scope: SoundExpressionScope,
    ) -> Self {
        let mut expression_graph = ExpressionGraph::new();

        // HACK: assuming 1 output for now
        expression_graph.add_result(default_value);

        Self {
            id,
            target_mapping: ExpressionParameterMapping::new(),
            expression_graph,
            owner,
            scope,
        }
    }

    pub(crate) fn id(&self) -> SoundExpressionId {
        self.id
    }

    pub(crate) fn parameter_mapping(&self) -> &ExpressionParameterMapping {
        debug_assert!(self
            .target_mapping
            .check_invariants(self.expression_graph.topology()));
        &self.target_mapping
    }

    pub(crate) fn expression_graph(&self) -> &ExpressionGraph {
        &self.expression_graph
    }

    pub(crate) fn expression_graph_mut(&mut self) -> &mut ExpressionGraph {
        &mut self.expression_graph
    }

    pub(crate) fn expression_graph_and_mapping_mut(
        &mut self,
    ) -> (&mut ExpressionGraph, &mut ExpressionParameterMapping) {
        (&mut self.expression_graph, &mut self.target_mapping)
    }

    pub(crate) fn owner(&self) -> SoundProcessorId {
        self.owner
    }

    pub(crate) fn scope(&self) -> &SoundExpressionScope {
        &self.scope
    }
}

impl Revisable for SoundExpressionData {
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = RevisionHasher::new();
        hasher.write_revisable(&self.id);
        hasher.write_revisable(&self.target_mapping);
        hasher.write_revisable(&self.expression_graph.topology());
        hasher.write_revisable(&self.owner);
        hasher.write_revisable(&self.scope);
        hasher.into_revision()
    }
}

#[derive(Clone)]
pub(crate) struct SoundExpressionArgumentData {
    id: SoundExpressionArgumentId,
    instance: Arc<dyn SoundExpressionArgument>,
    owner: SoundExpressionArgumentOwner,
}

impl SoundExpressionArgumentData {
    pub(crate) fn new(
        id: SoundExpressionArgumentId,
        instance: Arc<dyn SoundExpressionArgument>,
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

    pub(crate) fn instance_arc(&self) -> Arc<dyn SoundExpressionArgument> {
        Arc::clone(&self.instance)
    }

    pub(crate) fn owner(&self) -> SoundExpressionArgumentOwner {
        self.owner
    }
}

impl Revisable for SoundExpressionArgumentData {
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = RevisionHasher::new();
        hasher.write_revisable(&self.id);
        // Do not hash instance
        match &self.owner {
            SoundExpressionArgumentOwner::SoundProcessor(spid) => {
                hasher.write_u8(1);
                hasher.write_revisable(&spid);
            }
            SoundExpressionArgumentOwner::SoundInput(siid) => {
                hasher.write_u8(2);
                hasher.write_revisable(siid);
            }
        }
        hasher.into_revision()
    }
}
