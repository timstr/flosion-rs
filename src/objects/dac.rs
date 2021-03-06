use std::{
    sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        mpsc::{channel, Sender},
        Arc,
    },
    time::Duration,
};

use crate::core::{
    context::ProcessorContext,
    graphobject::{ObjectType, WithObjectType},
    resample::resample_interleave,
    samplefrequency::SAMPLE_FREQUENCY,
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundinput::{InputOptions, SingleSoundInputHandle},
    soundprocessor::StaticSoundProcessor,
    soundprocessortools::SoundProcessorTools,
    soundstate::SoundState,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleRate, Stream, StreamConfig, StreamError,
};
use parking_lot::Mutex;

struct StreamDammit {
    stream: Stream,
}

unsafe impl Send for StreamDammit {}

struct DacData {
    chunk_backlog: AtomicI32,
    playing: AtomicBool,
    first_chunk: AtomicBool,
}

pub struct Dac {
    input: SingleSoundInputHandle,
    stream: Mutex<StreamDammit>,
    chunk_sender: Mutex<Sender<SoundChunk>>,
    pending_reset: AtomicBool,
    shared_data: Arc<DacData>,
}

pub struct DacState {}

impl Default for DacState {
    fn default() -> DacState {
        DacState {}
    }
}

impl SoundState for DacState {
    fn reset(&mut self) {}
}

impl Dac {
    pub fn input(&self) -> &SingleSoundInputHandle {
        &self.input
    }

    pub fn is_playing(&self) -> bool {
        self.shared_data.playing.load(Ordering::SeqCst)
    }

    pub fn reset(&self) {
        self.pending_reset.store(true, Ordering::SeqCst);
    }
}

impl StaticSoundProcessor for Dac {
    type StateType = DacState;

    fn new_default(tools: &mut SoundProcessorTools<DacState>) -> Dac {
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
        // config.sample_rate = SampleRate(SAMPLE_FREQUENCY as u32);
        // config.buffer_size = BufferSize::Fixed(CHUNK_SIZE as u32);

        println!(
            "Requesting audio stream with {} channels, a {} Hz sample rate, and a buffer size of {:?}",
            config.channels, config.sample_rate.0, config.buffer_size
        );

        let (tx, rx) = channel::<SoundChunk>();

        let shared_data = Arc::new(DacData {
            playing: AtomicBool::new(false),
            chunk_backlog: AtomicI32::new(0),
            first_chunk: AtomicBool::new(false),
        });

        let mut chunk_index: usize = 0;
        let mut current_chunk: Option<SoundChunk> = None;
        let thread_shared_data = Arc::clone(&shared_data);

        let mut get_next_sample = move || {
            if current_chunk.is_none() || chunk_index >= CHUNK_SIZE {
                current_chunk = Some(loop {
                    if let Ok(mut next_chunk) = rx.try_recv() {
                        let backlog = thread_shared_data
                            .chunk_backlog
                            .fetch_sub(1, Ordering::SeqCst);
                        debug_assert!(backlog >= 0);
                        let init = thread_shared_data.first_chunk.swap(false, Ordering::SeqCst);
                        if init {
                            let mut dropped_chunk_count = 0;
                            loop {
                                if let Ok(backlog_chunk) = rx.try_recv() {
                                    let n = thread_shared_data
                                        .chunk_backlog
                                        .fetch_sub(1, Ordering::SeqCst);
                                    debug_assert!(n >= 0);
                                    if n == 0 {
                                        next_chunk = backlog_chunk;
                                    }
                                    dropped_chunk_count += 1;
                                } else {
                                    break;
                                }
                            }
                            if dropped_chunk_count > 1 {
                                println!("Warning! Dac was slow to start and {} initial chunks were dropped", dropped_chunk_count)
                            }
                        } else {
                            if backlog > 2 {
                                println!("Warning! Dac is behind by {} chunks", backlog);
                            }
                        }
                        break next_chunk;
                    }
                    if !thread_shared_data.playing.load(Ordering::Relaxed) {
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

        let data_callback = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            debug_assert!(data.len() % 2 == 0);
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

        Dac {
            input: tools.add_single_sound_input(InputOptions {
                realtime: true,
                interruptible: false,
            }),
            stream: Mutex::new(StreamDammit { stream }),
            chunk_sender: Mutex::new(tx),
            pending_reset: AtomicBool::new(false),
            shared_data,
        }
    }

    fn process_audio(&self, _dst: &mut SoundChunk, mut sc: ProcessorContext<'_, DacState>) {
        if self.pending_reset.swap(false, Ordering::SeqCst)
            || sc.single_input_needs_reset(&self.input)
        {
            sc.reset_single_input(&self.input, 0);
        }
        let mut ch = SoundChunk::new();
        sc.step_single_input(&self.input, &mut ch);

        let sender = self.chunk_sender.lock();
        sender.send(ch).unwrap();
        self.shared_data
            .chunk_backlog
            .fetch_add(1, Ordering::SeqCst);
    }

    fn produces_output(&self) -> bool {
        false
    }

    fn on_start_processing(&self) {
        self.shared_data.playing.store(true, Ordering::SeqCst);
        let s = self.stream.lock();
        s.stream.play().unwrap();
        self.shared_data.first_chunk.store(true, Ordering::SeqCst);
    }

    fn on_stop_processing(&self) {
        self.shared_data.playing.store(false, Ordering::SeqCst);
        let s = self.stream.lock();
        s.stream.pause().unwrap();
    }
}

impl WithObjectType for Dac {
    const TYPE: ObjectType = ObjectType::new("dac");
}
