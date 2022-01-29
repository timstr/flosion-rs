use flosion::{
    core::{soundgraph::SoundGraph, soundinput::SoundInputWrapper},
    objects::{
        dac::DAC,
        functions::{Constant, Sine, UnitSine},
        wavegenerator::WaveGenerator,
        whitenoise::WhiteNoise,
    },
};
use futures::executor::block_on;
use std::{thread, time::Duration};

async fn async_main() {
    let mut sg: SoundGraph = SoundGraph::new();
    // let wn = sg.add_dynamic_sound_processor::<WhiteNoise>().await;
    let wavegen = sg.add_dynamic_sound_processor::<WaveGenerator>().await;
    let dac = sg.add_static_sound_processor::<DAC>().await;
    let dac_input_id = dac.instance().input().id();
    let constant = sg.add_number_source::<Constant>().await;
    let usine = sg.add_number_source::<UnitSine>().await;
    sg.connect_number_input(wavegen.instance().amplitude.id(), usine.id())
        .await
        .unwrap();
    sg.connect_number_input(usine.instance().input.id(), wavegen.instance().phase.id())
        .await
        .unwrap();
    sg.connect_number_input(wavegen.instance().frequency.id(), constant.id())
        .await
        .unwrap();
    constant.instance().set_value(440.0);
    // println!("WhiteNoise id = {:?}", wn.id());
    println!("WaveGenerator id = {:?}", wavegen.id());
    println!("DAC id = {:?}", dac.id());
    println!("DAC input id = {:?}", dac.instance().input().id());
    println!("Before connecting:");
    // println!("WhiteNoise has {} states", wn.num_states());
    // println!("DAC has {} states", dac.num_states());
    sg.connect_sound_input(dac_input_id, wavegen.id())
        .await
        .unwrap();
    println!("After connecting:");
    // println!("WhiteNoise has {} states", wn.num_states());
    // println!("DAC has {} states", dac.num_states());

    println!("Starting audio processing");
    sg.start().await.unwrap();

    for _ in 0..16 {
        thread::sleep(Duration::from_millis(250));
        sg.disconnect_sound_input(dac_input_id).await.unwrap();
        thread::sleep(Duration::from_millis(250));
        sg.connect_sound_input(dac_input_id, wavegen.id())
            .await
            .unwrap();
    }

    println!("Stopping audio processing...");
    sg.stop().await.unwrap();
    println!("Stopping audio processing... Done.");

    sg.disconnect_sound_input(dac.instance().input().id())
        .await
        .unwrap();

    println!("After disconnecting:");
}

fn main() {
    block_on(async_main());
    println!("main() exiting");
}
