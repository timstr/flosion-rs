// TODO: consider renaming to e.g. Restartable
// TODO: or just remove
pub trait State: Send {
    fn start_over(&mut self);
}

impl State for () {
    fn start_over(&mut self) {}
}
