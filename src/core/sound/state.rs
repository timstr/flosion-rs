pub trait State: Sync + Send {
    fn start_over(&mut self);
}

impl State for () {
    fn start_over(&mut self) {}
}
