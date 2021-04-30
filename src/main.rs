use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rand::prelude::*;
use std::{thread, time};
// use cpal::Data;

fn main() {
    println!("Hello, world!");

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("No output device available");
    let mut supported_configs_range = device
        .supported_output_configs()
        .expect("Error while querying configs");
    let config = supported_configs_range
        .next()
        .expect("No supported config!?")
        .with_max_sample_rate()
        .into();

    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for s in data.iter_mut() {
                    let r: f32 = thread_rng().gen();
                    *s = 0.05 * r - 0.025;
                }
            },
            move |err| {
                println!("Error: {:?}", err);
            },
        )
        .unwrap();

    stream.play().unwrap();
    thread::sleep(time::Duration::from_secs(2));
    stream.pause().unwrap();
}
