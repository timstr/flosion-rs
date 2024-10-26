use std::collections::{HashMap, HashSet};

use hashstash::{
    InplaceUnstasher, Stashable, Stasher, UnstashError, Unstashable, UnstashableInplace,
};

use crate::core::{
    engine::{compiledexpression::CompiledExpression, soundgraphcompiler::SoundGraphCompiler},
    expression::{
        expressiongraph::{ExpressionGraph, ExpressionGraphParameterId},
        expressionobject::ExpressionObjectFactory,
    },
    uniqueid::UniqueId,
};

use super::{
    argument::{ProcessorArgumentId, ProcessorArgumentLocation},
    soundprocessor::{
        ProcessorComponent, ProcessorComponentVisitor, ProcessorComponentVisitorMut,
        SoundProcessorId,
    },
};

pub(crate) struct ProcessorExpressionTag;

pub(crate) type ProcessorExpressionId = UniqueId<ProcessorExpressionTag>;

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct ProcessorExpressionLocation {
    processor: SoundProcessorId,
    expression: ProcessorExpressionId,
}

impl ProcessorExpressionLocation {
    pub(crate) fn new(
        processor: SoundProcessorId,
        expression: ProcessorExpressionId,
    ) -> ProcessorExpressionLocation {
        ProcessorExpressionLocation {
            processor,
            expression,
        }
    }

    pub(crate) fn processor(&self) -> SoundProcessorId {
        self.processor
    }

    pub(crate) fn expression(&self) -> ProcessorExpressionId {
        self.expression
    }
}

#[derive(Clone)]
pub(crate) struct ExpressionParameterMapping {
    mapping: HashMap<ExpressionGraphParameterId, ProcessorArgumentLocation>,
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
    ) -> Option<ProcessorArgumentLocation> {
        self.mapping.get(&id).cloned()
    }

    pub(crate) fn parameter_from_argument(
        &self,
        id: ProcessorArgumentLocation,
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
        argument_id: ProcessorArgumentLocation,
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
        argument_id: ProcessorArgumentLocation,
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

    pub(crate) fn items(&self) -> &HashMap<ExpressionGraphParameterId, ProcessorArgumentLocation> {
        &self.mapping
    }
}

impl Stashable for ExpressionParameterMapping {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.array_of_proxy_objects(
            self.mapping.iter(),
            |(param_id, arg_loc), stasher| {
                stasher.u64(param_id.value() as _);
                stasher.u64(arg_loc.processor().value() as _);
                stasher.u64(arg_loc.argument().value() as _);
            },
            hashstash::Order::Unordered,
        );
    }
}

impl UnstashableInplace for ExpressionParameterMapping {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        let time_to_write = unstasher.time_to_write();

        if time_to_write {
            self.mapping.clear();
        }

        unstasher.array_of_proxy_objects(|unstasher| {
            let param_id = ExpressionGraphParameterId::new(unstasher.u64()? as _);
            let arg_loc = ProcessorArgumentLocation::new(
                SoundProcessorId::new(unstasher.u64()? as _),
                ProcessorArgumentId::new(unstasher.u64()? as _),
            );

            if time_to_write {
                self.mapping.insert(param_id, arg_loc);
            }

            Ok(())
        })?;

        Ok(())
    }
}

// TODO: make this shared by sound inputs too, and more ergonomic / self-enforcing
#[derive(Clone)]
pub struct SoundExpressionScope {
    available_arguments: Vec<ProcessorArgumentId>,
}

impl SoundExpressionScope {
    pub fn new_empty() -> SoundExpressionScope {
        SoundExpressionScope {
            available_arguments: Vec::new(),
        }
    }

    pub fn new(arguments: Vec<ProcessorArgumentId>) -> SoundExpressionScope {
        SoundExpressionScope {
            available_arguments: arguments,
        }
    }

    pub(crate) fn available_local_arguments(&self) -> &[ProcessorArgumentId] {
        &self.available_arguments
    }
}

impl Stashable for SoundExpressionScope {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.array_of_u64_iter(self.available_arguments.iter().map(|i| i.value() as u64));
    }
}

impl UnstashableInplace for SoundExpressionScope {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        let ids = unstasher.array_of_u64_iter()?;

        if unstasher.time_to_write() {
            self.available_arguments = ids.map(|i| ProcessorArgumentId::new(i as _)).collect();
        }

        Ok(())
    }
}

pub struct ProcessorExpression {
    id: ProcessorExpressionId,
    param_mapping: ExpressionParameterMapping,
    expression_graph: ExpressionGraph,
    scope: SoundExpressionScope,
}

impl ProcessorExpression {
    pub(crate) fn new(default_value: f32, scope: SoundExpressionScope) -> ProcessorExpression {
        let mut expression_graph = ExpressionGraph::new();

        // HACK assuming 1 output for now
        expression_graph.add_result(default_value);

        ProcessorExpression {
            id: ProcessorExpressionId::new_unique(),
            param_mapping: ExpressionParameterMapping::new(),
            expression_graph,
            scope,
        }
    }

    pub(crate) fn id(&self) -> ProcessorExpressionId {
        self.id
    }

    pub(crate) fn scope(&self) -> &SoundExpressionScope {
        &self.scope
    }

    pub(crate) fn mapping(&self) -> &ExpressionParameterMapping {
        &self.param_mapping
    }

    pub(crate) fn graph(&self) -> &ExpressionGraph {
        &self.expression_graph
    }

    pub(crate) fn graph_mut(&mut self) -> &mut ExpressionGraph {
        &mut self.expression_graph
    }

    pub(crate) fn parts_mut(&mut self) -> (&mut ExpressionParameterMapping, &mut ExpressionGraph) {
        (&mut self.param_mapping, &mut self.expression_graph)
    }

    pub(crate) fn add_argument(
        &mut self,
        argument_id: ProcessorArgumentLocation,
    ) -> ExpressionGraphParameterId {
        self.param_mapping
            .add_argument(argument_id, &mut self.expression_graph)
    }

    pub(crate) fn remove_argument(&mut self, argument_id: ProcessorArgumentLocation) {
        self.param_mapping
            .remove_argument(argument_id, &mut self.expression_graph);
    }
}

impl ProcessorComponent for ProcessorExpression {
    type CompiledType<'ctx> = CompiledExpression<'ctx>;

    fn visit<'a>(&self, visitor: &'a mut dyn ProcessorComponentVisitor) {
        visitor.expression(self);
    }

    fn visit_mut<'a>(&mut self, visitor: &'a mut dyn ProcessorComponentVisitorMut) {
        visitor.expression(self);
    }

    #[cfg(not(debug_assertions))]
    fn compile<'ctx>(
        &self,
        processor_id: SoundProcessorId,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> CompiledExpression<'ctx> {
        let function = compiler
            .get_compiled_expression(ProcessorExpressionLocation::new(processor_id, self.id))
            .unwrap();
        CompiledExpression::new(self.id, function)
    }

    #[cfg(debug_assertions)]
    fn compile<'ctx>(
        &self,
        processor_id: SoundProcessorId,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> CompiledExpression<'ctx> {
        // Pass scope to enable validation
        let function = compiler
            .get_compiled_expression(ProcessorExpressionLocation::new(processor_id, self.id))
            .unwrap();
        CompiledExpression::new(self.id, function, self.scope.clone())
    }
}

impl Stashable for ProcessorExpression {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.u64(self.id.value() as _);
        stasher.object(&self.param_mapping);
        stasher.object(&self.expression_graph);
        stasher.object(&self.scope);
    }
}

// TODO: allow passing extra context with HashStash trait
impl ProcessorExpression {
    pub(crate) fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher,
        expr_obj_factory: &ExpressionObjectFactory,
    ) -> Result<(), UnstashError> {
        let id = ProcessorExpressionId::new(unstasher.u64_always()? as _);
        if unstasher.time_to_write() {
            self.id = id;
        }
        unstasher.object_inplace(&mut self.param_mapping)?;

        // uhhhhhh
        let new_graph = unstasher
            .object_proxy(|unstasher| ExpressionGraph::unstash(unstasher, expr_obj_factory))?;
        if unstasher.time_to_write() {
            self.expression_graph = new_graph;
        }

        unstasher.object_inplace(&mut self.scope)?;
        Ok(())
    }
}
