use hashstash::{Stashable, Stasher};

use crate::ui_core::factories::Factories;

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
    // TODO: replace with just sound+expr object factories?
    factories: &'a Factories,
}

impl<'a> UnstashingContext<'a> {
    pub(crate) fn new(factories: &'a Factories) -> UnstashingContext<'a> {
        UnstashingContext { factories }
    }

    pub(crate) fn factories(&self) -> &'a Factories {
        self.factories
    }
}
