use crate::core::{
    engine::{compiledexpression::CompiledExpression, soundgraphcompiler::SoundGraphCompiler},
    uniqueid::UniqueId,
};

use super::{soundgraphdata::SoundExpressionScope, soundprocessor::SoundProcessorId};

pub struct SoundExpressionTag;

pub type SoundExpressionId = UniqueId<SoundExpressionTag>;

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
