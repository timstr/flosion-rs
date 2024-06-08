use crate::core::{
    engine::{compiledexpression::CompiledExpression, soundgraphcompiler::SoundGraphCompiler},
    uniqueid::UniqueId,
};

use super::{soundgraphdata::SoundExpressionScope, soundprocessor::SoundProcessorId};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SoundExpressionId(usize);

impl SoundExpressionId {
    pub(crate) fn new(id: usize) -> SoundExpressionId {
        SoundExpressionId(id)
    }
}

impl Default for SoundExpressionId {
    fn default() -> Self {
        SoundExpressionId(1)
    }
}

impl UniqueId for SoundExpressionId {
    fn value(&self) -> usize {
        self.0
    }

    fn next(&self) -> Self {
        SoundExpressionId(self.0 + 1)
    }
}

pub struct SoundExpressionHandle {
    id: SoundExpressionId,
    owner: SoundProcessorId,
    scope: SoundExpressionScope,
}

impl SoundExpressionHandle {
    // TODO: why is this pub?
    pub fn new(
        id: SoundExpressionId,
        owner: SoundProcessorId,
        scope: SoundExpressionScope,
    ) -> SoundExpressionHandle {
        SoundExpressionHandle { id, owner, scope }
    }

    pub fn id(&self) -> SoundExpressionId {
        self.id
    }

    pub fn owner(&self) -> SoundProcessorId {
        self.owner
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
        CompiledExpression::new(self.id, compiler, self.scope.clone())
    }
}
