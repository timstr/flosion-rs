use flosion::make_noise_for_two_seconds;
use flosion::sound::soundgraph::{SoundGraph, WhiteNoise, DAC};

fn main() {
    println!("Hello, world!");

    let mut sg: SoundGraph = SoundGraph::new();
    let wn = sg.add_dynamic_sound_processor(WhiteNoise::{});
    let dac = sg.add_static_sound_processor::<DAC>();
    sg.connect_input(wn.id(), dac.input().id());

    make_noise_for_two_seconds();
}
