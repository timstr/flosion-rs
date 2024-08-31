pub trait State: Sync + Send + 'static {
    fn start_over(&mut self);
}

impl State for () {
    fn start_over(&mut self) {}
}
