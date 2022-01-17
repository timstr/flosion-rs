use flosion::objects::dac::DAC;
use flosion::objects::whitenoise::WhiteNoise;
use flosion::sound::soundgraph::SoundGraph;
use std::time::Duration;

use std::thread;

use futures::executor::block_on;

async fn async_main() {
    let mut sg: SoundGraph = SoundGraph::new();
    let wn = sg.add_dynamic_sound_processor::<WhiteNoise>().await;
    let dac = sg.add_static_sound_processor::<DAC>().await;
    let dac_input_id = dac.instance().input().id();
    println!("WhiteNoise id = {:?}", wn.id());
    println!("DAC id = {:?}", dac.id());
    println!("DAC input id = {:?}", dac.instance().input().id());
    println!("Before connecting:");
    // println!("WhiteNoise has {} states", wn.num_states());
    // println!("DAC has {} states", dac.num_states());
    sg.connect_sound_input(dac_input_id, wn.id()).await.unwrap();
    println!("After connecting:");
    // println!("WhiteNoise has {} states", wn.num_states());
    // println!("DAC has {} states", dac.num_states());

    println!("Starting audio processing");
    sg.start().await.unwrap();

    for _ in 0..16 {
        thread::sleep(Duration::from_millis(250));
        sg.disconnect_sound_input(dac_input_id).await.unwrap();
        thread::sleep(Duration::from_millis(250));
        sg.connect_sound_input(dac_input_id, wn.id()).await.unwrap();
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
