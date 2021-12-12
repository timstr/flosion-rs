// use flosion::make_noise_for_two_seconds;
use flosion::objects::dac::DAC;
use flosion::objects::whitenoise::WhiteNoise;
use flosion::sound::soundgraph::SoundGraph;

fn main() {
    println!("Hello, world!");

    let mut sg: SoundGraph = SoundGraph::new();
    let wn = sg.add_dynamic_sound_processor::<WhiteNoise>();
    let dac = sg.add_static_sound_processor::<DAC>();
    println!("WhiteNoise id = {:?}", wn.id());
    println!("DAC id = {:?}", dac.id());
    println!("DAC input id = {:?}", dac.instance().input().id());
    println!("Before connecting:");
    println!("WhiteNoise has {} states", wn.num_states());
    println!("DAC has {} states", dac.num_states());
    sg.connect_sound_input(dac.instance().input().id(), wn.id())
        .unwrap();
    println!("After connecting:");
    println!("WhiteNoise has {} states", wn.num_states());
    println!("DAC has {} states", dac.num_states());

    sg.disconnect_sound_input(dac.instance().input().id())
        .unwrap();
    println!("After disconnecting:");
    println!("WhiteNoise has {} states", wn.num_states());
    println!("DAC has {} states", dac.num_states());

    // make_noise_for_two_seconds();
}
