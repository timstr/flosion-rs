use std::collections::{HashMap, HashSet};

use crate::core::{
    engine::{compiledexpression::CompiledExpression, soundgraphcompiler::SoundGraphCompiler},
    expression::expressiongraph::{ExpressionGraph, ExpressionGraphParameterId},
    uniqueid::UniqueId,
};

use super::{
    expressionargument::{ArgumentLocation, ProcessorArgumentId},
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
    mapping: HashMap<ExpressionGraphParameterId, ArgumentLocation>,
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
    ) -> Option<ArgumentLocation> {
        self.mapping.get(&id).cloned()
    }

    pub(crate) fn parameter_from_argument(
        &self,
        id: ArgumentLocation,
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
        argument_id: ArgumentLocation,
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
        argument_id: ArgumentLocation,
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

    pub(crate) fn items(&self) -> &HashMap<ExpressionGraphParameterId, ArgumentLocation> {
        &self.mapping
    }
}

#[derive(Clone)]
pub struct SoundExpressionScope {
    processor_state_available: bool,
    available_local_arguments: Vec<ProcessorArgumentId>,
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

    pub fn add_local(mut self, id: ProcessorArgumentId) -> SoundExpressionScope {
        self.available_local_arguments.push(id);
        self
    }

    pub(crate) fn processor_state_available(&self) -> bool {
        self.processor_state_available
    }

    pub(crate) fn available_local_arguments(&self) -> &[ProcessorArgumentId] {
        &self.available_local_arguments
    }
}

pub struct ProcessorExpression {
    id: ProcessorExpressionId,
    param_mapping: ExpressionParameterMapping,
    expression_graph: ExpressionGraph,
    scope: SoundExpressionScope,
}

impl ProcessorExpression {
    pub(crate) fn new(
        id: ProcessorExpressionId,
        scope: SoundExpressionScope,
        default_value: f32,
    ) -> ProcessorExpression {
        let mut expression_graph = ExpressionGraph::new();

        // HACK assuming 1 output for now
        expression_graph.add_result(default_value);

        ProcessorExpression {
            id,
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
        argument_id: ArgumentLocation,
    ) -> ExpressionGraphParameterId {
        self.param_mapping
            .add_argument(argument_id, &mut self.expression_graph)
    }

    pub(crate) fn remove_argument(&mut self, argument_id: ArgumentLocation) {
        self.param_mapping
            .remove_argument(argument_id, &mut self.expression_graph);
    }

    #[cfg(not(debug_assertions))]
    pub fn compile<'a, 'ctx>(
        &self,
        processor_id: SoundProcessorId,
        compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> CompiledExpression<'ctx> {
        let function = compiler.get_compiled_expression(processor_id, self);
        CompiledExpression::new(self.id, function)
    }

    #[cfg(debug_assertions)]
    pub fn compile<'a, 'ctx>(
        &self,
        processor_id: SoundProcessorId,
        compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> CompiledExpression<'ctx> {
        // Pass scope to enable validation
        let function = compiler.get_compiled_expression(processor_id, self);
        CompiledExpression::new(self.id, function, self.scope.clone())
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
        let function = compiler.get_compiled_expression(processor_id, self);
        CompiledExpression::new(self.id, function)
    }

    #[cfg(debug_assertions)]
    fn compile<'ctx>(
        &self,
        processor_id: SoundProcessorId,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> CompiledExpression<'ctx> {
        // Pass scope to enable validation
        let function = compiler.get_compiled_expression(processor_id, self);
        CompiledExpression::new(self.id, function, self.scope.clone())
    }
}
