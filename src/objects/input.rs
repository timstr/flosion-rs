use std::sync::{Arc, Barrier};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, SampleRate, StreamConfig,
};
use parking_lot::Mutex;
use ringbuffer::ReadResult;

use crate::core::{
    engine::nodegen::NodeGen,
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    samplefrequency::SAMPLE_FREQUENCY,
    sound::{
        context::Context,
        soundprocessor::{ProcessorTiming, StaticSoundProcessor, StaticSoundProcessorWithId},
        soundprocessortools::SoundProcessorTools,
    },
    soundchunk::{SoundChunk, CHUNK_SIZE},
};

pub struct Input {
    // TODO: how to do without mutex? In principle, only
    // the one state graph node corresponding to this
    // static processor will ever access this.
    chunk_receiver: Mutex<ringbuffer::Reader<SoundChunk>>,
    stream_end_barrier: Arc<Barrier>,
}

impl Input {
    pub fn get_buffer_reader(&self) -> ringbuffer::Reader<SoundChunk> {
        self.chunk_receiver.lock().clone()
    }
}

impl Drop for Input {
    fn drop(&mut self) {
        self.stream_end_barrier.wait();
    }
}

impl StaticSoundProcessor for Input {
    type SoundInputType = ();
    type NumberInputType<'ctx> = ();

    fn new(_tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| println!("No input device available"))?;

        println!("Selected input device {}", device.name().unwrap());

        let mut supported_configs_range = device
            .supported_input_configs()
            .expect("error while querying input configs");
        let supported_config = supported_configs_range
            .next()
            .expect("No supported input config:?")
            .with_sample_rate(SampleRate(SAMPLE_FREQUENCY as u32));
        let mut config: StreamConfig = supported_config.into();
        config.buffer_size = BufferSize::Fixed(CHUNK_SIZE as u32);

        // TODO: stereo???
        config.channels = 1;

        let mut current_chunk = SoundChunk::new();
        let mut chunk_cursor: usize = 0;

        // let (tx, rx) = sync_channel::<SoundChunk>(0);
        let (rx, mut tx) = ringbuffer::ring_buffer::<SoundChunk>(8);

        let data_callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            for sample in data {
                // TODO: stereo???
                current_chunk.l[chunk_cursor] = *sample;
                current_chunk.r[chunk_cursor] = *sample;
                chunk_cursor += 1;
                if chunk_cursor == CHUNK_SIZE {
                    chunk_cursor = 0;
                    tx.write(current_chunk);
                }
            }
        };

        let barrier = Arc::new(Barrier::new(2));
        let barrier2 = Arc::clone(&barrier);

        // NOTE: Stream is not Send, using a dedicated thread as a workaround
        std::thread::spawn(move || {
            println!(
                "Requesting input audio stream with {} channels, a {} Hz sample rate, and a buffer size of {:?}",
                config.channels, config.sample_rate.0, config.buffer_size
            );

            let stream = device
                .build_input_stream(&config, data_callback, |err| {
                    panic!("CPAL Input stream encountered an error: {}", err);
                })
                .unwrap();
            stream.play().unwrap();
            barrier2.wait();
            stream.pause().unwrap();
        });

        Ok(Input {
            chunk_receiver: Mutex::new(rx),
            stream_end_barrier: barrier,
        })
    }

    fn get_sound_input(&self) -> &() {
        &()
    }

    fn make_number_inputs<'a, 'ctx>(
        &self,
        _nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self::NumberInputType<'ctx> {
        ()
    }

    fn process_audio<'ctx>(
        processor: &StaticSoundProcessorWithId<Self>,
        _timing: &ProcessorTiming,
        _sound_inputs: &mut (),
        _number_inputs: &mut (),
        dst: &mut SoundChunk,
        _context: Context,
    ) {
        let chunk = match processor.chunk_receiver.lock().read() {
            ReadResult::Ok(ch) => ch,
            ReadResult::Dropout(ch) => {
                println!("WARNING: Input dropout");
                ch
            }
            ReadResult::Empty => return,
        };
        *dst = chunk;
    }
}

impl WithObjectType for Input {
    const TYPE: ObjectType = ObjectType::new("input");
}
