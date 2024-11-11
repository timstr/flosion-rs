use std::collections::{HashMap, HashSet};

use hashstash::{
    InplaceUnstasher, Stashable, Stasher, UnstashError, Unstashable, UnstashableInplace, Unstasher,
};

use crate::core::{
    engine::{compiledexpression::CompiledExpression, soundgraphcompiler::SoundGraphCompiler},
    expression::expressiongraph::{ExpressionGraph, ExpressionGraphParameterId},
    stashing::{StashingContext, UnstashingContext},
    uniqueid::UniqueId,
};

use super::{
    argument::{ArgumentScope, ProcessorArgumentLocation},
    soundinput::SoundInputLocation,
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

impl Stashable for ProcessorExpressionLocation {
    fn stash(&self, stasher: &mut Stasher) {
        self.processor.stash(stasher);
        self.expression.stash(stasher);
    }
}

impl Unstashable for ProcessorExpressionLocation {
    fn unstash(unstasher: &mut Unstasher) -> Result<Self, UnstashError> {
        Ok(ProcessorExpressionLocation {
            processor: SoundProcessorId::unstash(unstasher)?,
            expression: ProcessorExpressionId::unstash(unstasher)?,
        })
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub(crate) enum ExpressionParameterTarget {
    Argument(ProcessorArgumentLocation),
    ProcessorTime(SoundProcessorId),
    InputTime(SoundInputLocation),
}

impl Stashable for ExpressionParameterTarget {
    fn stash(&self, stasher: &mut Stasher) {
        match self {
            ExpressionParameterTarget::Argument(arg_loc) => {
                stasher.u8(0);
                arg_loc.stash(stasher);
            }
            ExpressionParameterTarget::ProcessorTime(spid) => {
                stasher.u8(1);
                spid.stash(stasher);
            }
            ExpressionParameterTarget::InputTime(input_loc) => {
                stasher.u8(2);
                input_loc.stash(stasher);
            }
        }
    }
}

impl Unstashable for ExpressionParameterTarget {
    fn unstash(unstasher: &mut Unstasher) -> Result<Self, UnstashError> {
        Ok(match unstasher.u8()? {
            0 => {
                ExpressionParameterTarget::Argument(ProcessorArgumentLocation::unstash(unstasher)?)
            }
            1 => ExpressionParameterTarget::ProcessorTime(SoundProcessorId::unstash(unstasher)?),
            2 => ExpressionParameterTarget::InputTime(SoundInputLocation::unstash(unstasher)?),
            _ => panic!(),
        })
    }
}

#[derive(Clone)]
pub(crate) struct ExpressionParameterMapping {
    mapping: HashMap<ExpressionGraphParameterId, ExpressionParameterTarget>,
}

impl ExpressionParameterMapping {
    pub(crate) fn new() -> ExpressionParameterMapping {
        ExpressionParameterMapping {
            mapping: HashMap::new(),
        }
    }

    pub(crate) fn target_from_parameter(
        &self,
        id: ExpressionGraphParameterId,
    ) -> Option<ExpressionParameterTarget> {
        self.mapping.get(&id).cloned()
    }

    pub(crate) fn parameter_from_target(
        &self,
        target: ExpressionParameterTarget,
    ) -> Option<ExpressionGraphParameterId> {
        for (giid, t) in &self.mapping {
            if *t == target {
                return Some(*giid);
            }
        }
        None
    }

    // NOTE: passing ExpressionGraph separately here might seem a bit awkward from the perspective of the
    // SoundExpressionData that owns this and the expression graph, but it makes the two separable.
    // This is useful for making LexicalLayout more reusable accross different types of expressions
    pub(crate) fn add_target(
        &mut self,
        target: ExpressionParameterTarget,
        expr_graph: &mut ExpressionGraph,
    ) -> ExpressionGraphParameterId {
        debug_assert!(self.check_invariants(expr_graph));
        if let Some(giid) = self.parameter_from_target(target) {
            return giid;
        }
        let giid = expr_graph.add_parameter();
        let prev = self.mapping.insert(giid, target);
        debug_assert_eq!(prev, None);
        debug_assert!(self.check_invariants(expr_graph));
        giid
    }

    pub(crate) fn remove_target(
        &mut self,
        target: ExpressionParameterTarget,
        expr_graph: &mut ExpressionGraph,
    ) {
        debug_assert!(self.check_invariants(expr_graph));
        let giid = self.parameter_from_target(target).unwrap();
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

    pub(crate) fn items(&self) -> &HashMap<ExpressionGraphParameterId, ExpressionParameterTarget> {
        &self.mapping
    }
}

impl Stashable for ExpressionParameterMapping {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.array_of_proxy_objects(
            self.mapping.iter(),
            |(param_id, target), stasher| {
                stasher.u64(param_id.value() as _);
                target.stash(stasher);
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
            let arg_loc = ExpressionParameterTarget::unstash(unstasher)?;

            if time_to_write {
                self.mapping.insert(param_id, arg_loc);
            }

            Ok(())
        })?;

        Ok(())
    }
}

pub struct ProcessorExpression {
    id: ProcessorExpressionId,
    param_mapping: ExpressionParameterMapping,
    expression_graph: ExpressionGraph,
    scope: ArgumentScope,
}

impl ProcessorExpression {
    pub(crate) fn new(default_value: f32, scope: ArgumentScope) -> ProcessorExpression {
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

    pub(crate) fn scope(&self) -> &ArgumentScope {
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

    pub(crate) fn add_target(
        &mut self,
        target: ExpressionParameterTarget,
    ) -> ExpressionGraphParameterId {
        self.param_mapping
            .add_target(target, &mut self.expression_graph)
    }

    pub(crate) fn remove_target(&mut self, target: ExpressionParameterTarget) {
        self.param_mapping
            .remove_target(target, &mut self.expression_graph);
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

impl Stashable<StashingContext> for ProcessorExpression {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.u64(self.id.value() as _);
        stasher.object_with_context(&self.param_mapping, ());
        stasher.object(&self.expression_graph);
        stasher.object(&self.scope);
    }
}

impl<'a> UnstashableInplace<UnstashingContext<'a>> for ProcessorExpression {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext>,
    ) -> Result<(), UnstashError> {
        let id = ProcessorExpressionId::new(unstasher.u64_always()? as _);
        if unstasher.time_to_write() {
            self.id = id;
        }
        unstasher.object_inplace_with_context(&mut self.param_mapping, ())?;
        unstasher.object_inplace(&mut self.expression_graph)?;
        unstasher.object_inplace(&mut self.scope)?;
        Ok(())
    }
}
