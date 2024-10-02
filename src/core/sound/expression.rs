use crate::core::{
    engine::{compiledexpression::CompiledExpression, soundgraphcompiler::SoundGraphCompiler},
    expression::expressiongraph::{ExpressionGraph, ExpressionGraphParameterId},
    uniqueid::UniqueId,
};

use super::{
    expressionargument::SoundExpressionArgumentId,
    soundgraphdata::{ExpressionParameterMapping, SoundExpressionScope},
    soundprocessor::SoundProcessorId,
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
        argument_id: SoundExpressionArgumentId,
    ) -> ExpressionGraphParameterId {
        self.param_mapping
            .add_argument(argument_id, &mut self.expression_graph)
    }

    pub(crate) fn remove_argument(&mut self, argument_id: SoundExpressionArgumentId) {
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
