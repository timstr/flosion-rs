use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{sync_channel, SyncSender, TrySendError},
    Arc, Barrier,
};

use crate::core::{
    context::Context,
    graphobject::{ObjectType, WithObjectType},
    resample::resample_interleave,
    samplefrequency::SAMPLE_FREQUENCY,
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundinput::InputOptions,
    soundprocessor::{SoundProcessor, StreamStatus},
    soundprocessortools::SoundProcessorTools,
    statetree::{ProcessorState, SingleInput, SingleInputNode, State},
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleRate, StreamConfig, StreamError,
};

pub struct DacData {
    stream_end_barrier: Barrier,
    pending_reset: AtomicBool,
    chunk_sender: SyncSender<SoundChunk>,
}

pub struct Dac {
    pub input: SingleInput,
    shared_data: Arc<DacData>,
}

impl State for Arc<DacData> {
    fn reset(&mut self) {
        // Nothing to do
    }
}

impl Dac {
    pub fn reset(&self) {
        self.shared_data.pending_reset.store(true, Ordering::SeqCst);
    }
}

impl SoundProcessor for Dac {
    const IS_STATIC: bool = true;

    type StateType = Arc<DacData>;

    type InputType = SingleInput;

    fn new(mut tools: SoundProcessorTools) -> Dac {
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

        let (tx, rx) = sync_channel::<SoundChunk>(0);

        let shared_data = Arc::new(DacData {
            chunk_sender: tx,
            pending_reset: AtomicBool::new(false),
            stream_end_barrier: Barrier::new(2),
        });

        let mut chunk_index: usize = 0;
        let mut current_chunk: Option<SoundChunk> = None;

        let mut get_next_sample = move || {
            if current_chunk.is_none() || chunk_index >= CHUNK_SIZE {
                current_chunk = Some(rx.recv().unwrap_or_else(|_| SoundChunk::new()));
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

        let shared_data_also = Arc::clone(&shared_data);

        std::thread::spawn(move || {
            let stream = device
                .build_output_stream(&config, data_callback, err_callback)
                .unwrap();
            stream.play().unwrap();
            shared_data_also.stream_end_barrier.wait();
            stream.pause().unwrap();
        });

        Dac {
            input: SingleInput::new(
                InputOptions {
                    realtime: true,
                    interruptible: false,
                },
                &mut tools,
            ),
            shared_data,
        }
    }

    fn get_input(&self) -> &SingleInput {
        &self.input
    }

    fn make_state(&self) -> Arc<DacData> {
        Arc::clone(&self.shared_data)
    }

    fn process_audio(
        state: &mut ProcessorState<Arc<DacData>>,
        input: &mut SingleInputNode,
        _dst: &mut SoundChunk,
        ctx: Context,
    ) -> StreamStatus {
        if input.needs_reset() || state.pending_reset.swap(false, Ordering::SeqCst) {
            input.reset(0);
        }
        let mut ch = SoundChunk::new();
        input.step(state, &mut ch, &ctx);

        if let Err(e) = state.chunk_sender.try_send(ch) {
            match e {
                TrySendError::Full(_) => println!("Dac dropped a chunk"),
                TrySendError::Disconnected(_) => panic!("Idk what to do, maybe nothing?"),
            }
        }
        StreamStatus::StaticNoOutput
    }
}

// impl Drop for Dac {
//     fn drop(&mut self) {
//         println!("Dropping Dac");
//         self.shared_data.stream_end_barrier.wait();
//         self.stream_thread.take().unwrap().join().unwrap();
//     }
// }

impl WithObjectType for Dac {
    const TYPE: ObjectType = ObjectType::new("dac");
}
