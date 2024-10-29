use hashstash::{Stashable, Stasher};

use crate::ui_core::flosion_ui::Factories;

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

pub struct UnstashingContext<'a> {
    factories: &'a Factories,
}

impl<'a> UnstashingContext<'a> {
    pub(crate) fn new(factories: &'a Factories) -> UnstashingContext<'a> {
        UnstashingContext { factories }
    }

    pub(crate) fn factories(&self) -> &Factories {
        self.factories
    }
}
