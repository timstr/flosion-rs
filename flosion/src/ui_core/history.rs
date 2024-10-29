use hashstash::{Stash, StashHandle};

use super::appstate::AppState;

pub(crate) struct History {
    snapshots: Vec<StashHandle<AppState>>,
    current_snapshot: usize,
}

impl History {
    pub(crate) fn new() -> History {
        History {
            snapshots: Vec::new(),
            current_snapshot: 0,
        }
    }

    pub(crate) fn push_snapshot(&mut self, stash: &Stash, app_state: &AppState) {
        if self.snapshots.is_empty() {
            debug_assert_eq!(self.current_snapshot, 0);
            self.snapshots.push(stash.stash(app_state));
        }
        self.snapshots.truncate(self.current_snapshot);
    }

    pub(crate) fn undo(&mut self, stash: &Stash, app_state: &mut AppState) {
        if self.snapshots.is_empty() {
            return;
        }

        if self.current_snapshot > 0 {
            self.current_snapshot -= 1;
            stash
                .unstash_inplace(&self.snapshots[self.current_snapshot], app_state)
                .unwrap();
        }
    }

    pub(crate) fn redo(&mut self, stash: &Stash, app_state: &mut AppState) {
        if self.snapshots.is_empty() {
            return;
        }

        if self.current_snapshot + 1 < self.snapshots.len() {
            self.current_snapshot += 1;
            stash
                .unstash_inplace(&self.snapshots[self.current_snapshot], app_state)
                .unwrap();
        }
    }
}
