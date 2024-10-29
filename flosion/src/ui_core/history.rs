use hashstash::StashHandle;

use super::appstate::AppState;

pub(crate) struct History {
    snapshots: Vec<StashHandle<AppState>>,
    current_snapshot: usize,
}
