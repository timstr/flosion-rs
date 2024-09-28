use crate::core::{
    engine::{compiledexpression::CompiledExpression, soundgraphcompiler::SoundGraphCompiler},
    expression::expressiongraph::ExpressionGraph,
};

use super::soundgraphdata::{ExpressionParameterMapping, SoundExpressionScope};

// TODO: rename just SoundExpression?
pub struct SoundExpressionHandle {
    param_mapping: ExpressionParameterMapping,
    expression_graph: ExpressionGraph,
    scope: SoundExpressionScope,
    default_value: f32,
}

impl SoundExpressionHandle {
    // TODO: why is this pub?
    pub fn new(scope: SoundExpressionScope, default_value: f32) -> SoundExpressionHandle {
        SoundExpressionHandle {
            param_mapping: ExpressionParameterMapping::new(),
            expression_graph: ExpressionGraph::new(),
            scope,
            default_value,
        }
    }

    #[cfg(not(debug_assertions))]
    pub fn compile<'a, 'ctx>(
        &self,
        compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> CompiledExpression<'ctx> {
        CompiledExpression::new(self.id, compiler)
    }

    #[cfg(debug_assertions)]
    pub fn compile<'a, 'ctx>(
        &self,
        compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> CompiledExpression<'ctx> {
        // Pass scope to enable validation
        // CompiledExpression::new(self.id, compiler, self.scope.clone())
        todo!()
    }
}
