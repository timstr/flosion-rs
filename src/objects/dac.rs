use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Sender, TryRecvError},
        Arc, Mutex,
    },
    time::Duration,
};

use crate::sound::{
    context::Context,
    resample::resample_interleave,
    samplefrequency::SAMPLE_FREQUENCY,
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundinput::{InputOptions, SingleSoundInputHandle},
    soundprocessor::StaticSoundProcessor,
    soundprocessortools::SoundProcessorTools,
    soundstate::{SoundState, StateTime},
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, SampleRate, Stream, StreamConfig, StreamError,
};
use thread_priority::{set_current_thread_priority, ThreadPriority};

struct StreamDammit {
    stream: Stream,
}

unsafe impl Send for StreamDammit {}

pub struct DAC {
    input: SingleSoundInputHandle,
    stream: Mutex<StreamDammit>,
    chunk_sender: Mutex<Sender<SoundChunk>>,
    playing: Arc<AtomicBool>,
}

pub struct DACState {
    time: StateTime,
}

impl Default for DACState {
    fn default() -> DACState {
        DACState {
            time: StateTime::new(),
        }
    }
}

impl SoundState for DACState {
    fn reset(&mut self) {}

    fn time(&self) -> &StateTime {
        &self.time
    }

    fn time_mut(&mut self) -> &mut StateTime {
        &mut self.time
    }
}

impl DAC {
    pub fn input(&self) -> &SingleSoundInputHandle {
        &self.input
    }
}

impl StaticSoundProcessor for DAC {
    type StateType = DACState;

    fn new(tools: &mut SoundProcessorTools) -> DAC {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("No output device available");
        println!("Using output device {}", device.name().unwrap());
        let supported_configs = device
            .supported_output_configs()
            .expect("Error while querying configs")
            .next()
            .expect("No supported config!?");

        println!(
            "Supported sample rates are {:?} to {:?}",
            supported_configs.min_sample_rate().0,
            supported_configs.max_sample_rate().0
        );

        println!(
            "Supported buffer sizes are {:?}",
            supported_configs.buffer_size()
        );

        let sample_rate = SampleRate(supported_configs.min_sample_rate().0.max(44_100));
        let mut config: StreamConfig = supported_configs.with_sample_rate(sample_rate).into();

        config.channels = 2;
        config.sample_rate = SampleRate(SAMPLE_FREQUENCY as u32);
        // config.buffer_size = BufferSize::Fixed(CHUNK_SIZE as u32);

        println!(
            "Requesting audio stream with {} channels, a {} Hz sample rate, and a buffer size of {:?}",
            config.channels, config.sample_rate.0, config.buffer_size
        );

        let (tx, rx) = channel::<SoundChunk>();

        let playing = Arc::new(AtomicBool::new(false));
        let playing_also = Arc::clone(&playing);

        let mut chunk_index: usize = 0;
        let mut current_chunk: Option<SoundChunk> = None;

        let mut get_next_sample = move || {
            if current_chunk.is_none() || chunk_index >= CHUNK_SIZE {
                // current_chunk = Some(if let Ok(b) = rx.try_recv() {
                //     b
                // } else {
                //     // println!("CPAL thread blocking");
                //     // rx.recv().unwrap()

                //     println!("CPAL thread received no audio, producing silence instead");
                //     SoundChunk::new()
                // });
                current_chunk = Some(loop {
                    if let Ok(b) = rx.try_recv() {
                        break b;
                    }
                    if !playing.load(Ordering::Relaxed) {
                        println!("Playback has stopped, producing silence");
                        break SoundChunk::new();
                    }
                    spin_sleep::sleep(Duration::from_micros(1_000));
                });
                chunk_index = 0;
            }
            let c = current_chunk.as_ref().unwrap();
            let l = c.l[chunk_index];
            let r = c.r[chunk_index];
            chunk_index += 1;
            (l, r)
        };

        let mut init = false;

        let data_callback = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            assert!(data.len() % 2 == 0);
            if !init {
                // set_current_thread_priority(ThreadPriority::Max).unwrap();
                // spin_sleep::sleep(Duration::from_micros(10_000));
                init = true;
            }
            // println!("CPAL asked for {} samples", (data.len() / 2));
            resample_interleave(
                data,
                || get_next_sample(),
                SAMPLE_FREQUENCY as u32,
                sample_rate.0,
            );
        };

        let err_callback = |err: StreamError| {
            println!("CPAL StreamError: {:?}", err);
        };

        let stream = device
            .build_output_stream(&config, data_callback, err_callback)
            .unwrap();

        DAC {
            input: tools.add_single_input(InputOptions {
                realtime: true,
                interruptible: false,
            }),
            stream: Mutex::new(StreamDammit { stream }),
            chunk_sender: Mutex::new(tx),
            playing: playing_also,
        }
    }

    fn process_audio(&self, _state: &mut DACState, context: &mut Context) {
        let mut ch = SoundChunk::new();
        context.step_single_input(&self.input, &mut ch);

        let sender = self.chunk_sender.lock().unwrap();
        sender.send(ch).unwrap();
    }

    fn produces_output(&self) -> bool {
        false
    }

    fn on_start_processing(&self) {
        println!("CPAL thread on_start_processing");
        self.playing.store(true, Ordering::SeqCst);
        let s = self.stream.lock().unwrap();
        s.stream.play().unwrap();
    }

    fn on_stop_processing(&self) {
        println!("CPAL thread on_stop_processing");
        self.playing.store(false, Ordering::SeqCst);
        let s = self.stream.lock().unwrap();
        s.stream.pause().unwrap();
    }
}
