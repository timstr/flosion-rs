use crate::core::sound::{
    soundgraphtopology::SoundGraphTopology, soundgraphvalidation::find_sound_error,
};

#[test]
fn find_error_empty_graph() {
    let desc = SoundGraphTopology::new();
    let e = find_sound_error(&desc);
    assert!(e.is_none());
}
