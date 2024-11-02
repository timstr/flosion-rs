use std::cell::Cell;

use hashstash::{ObjectHash, Stash, StashHandle};

use crate::core::{
    sound::soundgraph::SoundGraph,
    stashing::{StashingContext, UnstashingContext},
};

use super::{appstate::AppState, factories::Factories, stashing::UiUnstashingContext};

struct Snapshot {
    graph: StashHandle<SoundGraph>,
    app_state: StashHandle<AppState>,
}

pub(crate) struct History {
    snapshots: Vec<Snapshot>,
    current_snapshot: usize,
}

impl History {
    pub(crate) fn new() -> History {
        History {
            snapshots: Vec::new(),
            current_snapshot: 0,
        }
    }

    pub(crate) fn push_snapshot(
        &mut self,
        stash: &Stash,
        graph: &SoundGraph,
        app_state: &AppState,
    ) {
        let snapshot = Snapshot {
            graph: stash.stash_with_context(graph, StashingContext::new_stashing_normally()),
            app_state: stash.stash(app_state),
        };
        if self.snapshots.is_empty() {
            debug_assert_eq!(self.current_snapshot, 0);
            self.snapshots.push(snapshot);
            return;
        }
        debug_assert!(self.current_snapshot < self.snapshots.len());
        self.current_snapshot += 1;
        self.snapshots.truncate(self.current_snapshot);
        self.snapshots.push(snapshot);
    }

    pub(crate) fn undo(
        &mut self,
        stash: &Stash,
        factories: &Factories,
        graph: &mut SoundGraph,
        app_state: &mut AppState,
    ) {
        if self.snapshots.is_empty() {
            return;
        }

        if self.current_snapshot > 0 {
            self.current_snapshot -= 1;
            Self::restore_from_snapshot(
                &self.snapshots[self.current_snapshot],
                stash,
                factories,
                graph,
                app_state,
            );
        }
    }

    pub(crate) fn redo(
        &mut self,
        stash: &Stash,
        factories: &Factories,
        graph: &mut SoundGraph,
        app_state: &mut AppState,
    ) {
        if self.snapshots.is_empty() {
            return;
        }

        if self.current_snapshot + 1 < self.snapshots.len() {
            self.current_snapshot += 1;
            Self::restore_from_snapshot(
                &self.snapshots[self.current_snapshot],
                stash,
                factories,
                graph,
                app_state,
            );
        }
    }

    fn restore_from_snapshot(
        snapshot: &Snapshot,
        stash: &Stash,
        factories: &Factories,
        graph: &mut SoundGraph,
        app_state: &mut AppState,
    ) {
        stash
            .unstash_inplace_with_context(&snapshot.graph, graph, UnstashingContext::new(factories))
            .unwrap();

        debug_assert_eq!(
            ObjectHash::from_stashable_and_context(graph, StashingContext::new_stashing_normally()),
            snapshot.graph.object_hash()
        );

        // NOTE: the app state is unstashed separately from the graph itself
        // because parts of the graph must exist before the app state can
        // be unstashed. In the two-phase in-place unstashing workflow, newly-
        // unstashed graph objects would be missing during the first phase if
        // the app state was being unstashed at the same time.
        stash
            .unstash_inplace_with_context(
                &snapshot.app_state,
                app_state,
                UiUnstashingContext::new(factories, graph),
            )
            .unwrap();

        debug_assert_eq!(
            ObjectHash::from_stashable(app_state),
            snapshot.app_state.object_hash()
        )
    }
}

pub(crate) struct SnapshotFlag {
    snapshot_requested: Cell<bool>,
}

impl SnapshotFlag {
    pub(crate) fn new() -> SnapshotFlag {
        SnapshotFlag {
            snapshot_requested: Cell::new(false),
        }
    }

    pub(crate) fn request_snapshot(&self) {
        self.snapshot_requested.set(true);
    }

    pub(crate) fn snapshot_was_requested(&self) -> bool {
        self.snapshot_requested.get()
    }
}
