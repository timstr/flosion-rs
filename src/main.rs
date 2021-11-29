use flosion::make_noise_for_two_seconds;
use flosion::objects::dac::DAC;
use flosion::objects::whitenoise::WhiteNoise;
use flosion::sound::soundgraph::SoundGraph;

fn main() {
    println!("Hello, world!");

    let mut sg: SoundGraph = SoundGraph::new();
    let wn = sg.add_dynamic_sound_processor::<WhiteNoise>();
    let dac = sg.add_static_sound_processor::<DAC>();
    sg.connect_input(dac.borrow().input().id(), wn.borrow().id());

    make_noise_for_two_seconds();
}
