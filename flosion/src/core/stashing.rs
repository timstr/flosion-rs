use hashstash::{Stashable, Stasher};

use super::{
    expression::expressionobject::ExpressionObjectFactory, sound::soundobject::SoundObjectFactory,
};

#[derive(Copy, Clone)]
pub struct StashingContext {
    checking_recompilation: bool,
}

impl StashingContext {
    pub(crate) fn new_stashing_normally() -> StashingContext {
        StashingContext {
            checking_recompilation: false,
        }
    }

    pub(crate) fn new_checking_recompilation() -> StashingContext {
        StashingContext {
            checking_recompilation: true,
        }
    }

    pub fn checking_recompilation(&self) -> bool {
        self.checking_recompilation
    }
}

impl Stashable<()> for StashingContext {
    fn stash(&self, stasher: &mut Stasher<()>) {
        stasher.bool(self.checking_recompilation);
    }
}

#[derive(Copy, Clone)]
pub struct UnstashingContext<'a> {
    sound_object_factory: &'a SoundObjectFactory,
    expression_object_factory: &'a ExpressionObjectFactory,
}

impl<'a> UnstashingContext<'a> {
    pub(crate) fn new(
        sound_object_factory: &'a SoundObjectFactory,
        expression_object_factory: &'a ExpressionObjectFactory,
    ) -> UnstashingContext<'a> {
        UnstashingContext {
            sound_object_factory,
            expression_object_factory,
        }
    }

    pub(crate) fn sound_object_factory(&self) -> &'a SoundObjectFactory {
        self.sound_object_factory
    }

    pub(crate) fn expression_object_factory(&self) -> &'a ExpressionObjectFactory {
        self.expression_object_factory
    }
}

#[derive(Copy, Clone)]
pub struct ExpressionUnstashingContext<'a> {
    expression_object_factory: &'a ExpressionObjectFactory,
}

impl<'a> ExpressionUnstashingContext<'a> {
    pub(crate) fn new(
        expression_object_factory: &'a ExpressionObjectFactory,
    ) -> ExpressionUnstashingContext<'a> {
        ExpressionUnstashingContext {
            expression_object_factory,
        }
    }

    pub(crate) fn expression_object_factory(&self) -> &'a ExpressionObjectFactory {
        self.expression_object_factory
    }
}
