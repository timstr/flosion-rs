// TODO: consider renaming to e.g. Restartable
// TODO: remove Sync (keep Send)
pub trait State: Sync + Send {
    fn start_over(&mut self);
}

impl State for () {
    fn start_over(&mut self) {}
}
