use flosion::make_noise_for_two_seconds;
use flosion::sound::soundgraph::{SoundGraph, WhiteNoise, DAC};

fn main() {
    println!("Hello, world!");

    let mut sg: SoundGraph = SoundGraph::new();
    let wn_id = sg.add_dynamic_sound_processor(WhiteNoise {});
    let dac_id = sg.add_static_sound_processor(DAC {});
    sg.connect_input(wn_id, dac_id);

    make_noise_for_two_seconds();
}
