use hashstash::StashHandle;

pub(crate) struct History {
    snapshots: Vec<StashHandle<()>>,
}
