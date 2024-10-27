use hashstash::{Stashable, Stasher};

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

impl Stashable for StashingContext {
    type Context = ();

    fn stash(&self, stasher: &mut Stasher<()>) {
        stasher.bool(self.checking_recompilation);
    }
}
