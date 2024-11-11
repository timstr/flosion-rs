use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{sync_channel, Receiver, SyncSender, TrySendError},
    Arc, Barrier,
};

use crate::{
    core::{
        objecttype::{ObjectType, WithObjectType},
        resample::resample_interleave,
        samplefrequency::SAMPLE_FREQUENCY,
        sound::{
            argument::ArgumentScope,
            context::Context,
            inputtypes::singleinput::SingleInput,
            soundinput::{InputContext, InputOptions},
            soundprocessor::{
                ProcessorState, SoundProcessor, StartOver, StateMarker, StreamStatus,
            },
        },
        soundchunk::{SoundChunk, CHUNK_SIZE},
        stashing::{StashingContext, UnstashingContext},
    },
    ui_core::arguments::ParsedArguments,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleRate, StreamConfig, StreamError,
};
use flosion_macros::ProcessorComponents;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};
use parking_lot::Mutex;

pub struct OutputData {
    pending_startover: AtomicBool,
    chunk_sender: SyncSender<SoundChunk>,
    // TODO: improve this
    chunk_receiver: Mutex<Receiver<SoundChunk>>,
}

// TODO: rename to e.g. "SoundOut", "Output" is too vague and overloaded
// AudioOut?
#[derive(ProcessorComponents)]
pub struct Output {
    pub input: SingleInput,

    #[not_a_component]
    shared_data: Arc<OutputData>,

    #[state]
    state: StateMarker<OutputState>,
}

impl Output {
    pub fn start_over(&self) {
        self.shared_data
            .pending_startover
            .store(true, Ordering::SeqCst);
    }
}

pub struct OutputState {
    shared_data: Arc<OutputData>,
    stream_end_barrier: Arc<Barrier>,
}

impl StartOver for OutputState {
    fn start_over(&mut self) {
        // ???
    }
}

impl Drop for OutputState {
    fn drop(&mut self) {
        self.stream_end_barrier.wait();
    }
}

impl SoundProcessor for Output {
    fn new(_args: &ParsedArguments) -> Output {
        let (tx, rx) = sync_channel::<SoundChunk>(0);

        let shared_data = Arc::new(OutputData {
            pending_startover: AtomicBool::new(false),
            chunk_sender: tx,
            chunk_receiver: Mutex::new(rx),
        });

        Output {
            input: SingleInput::new(InputOptions::Synchronous, ArgumentScope::new_empty()),
            shared_data,
            state: StateMarker::new(),
        }
    }

    fn is_static(&self) -> bool {
        true
    }

    fn process_audio(
        output: &mut CompiledOutput,
        dst: &mut SoundChunk,
        context: &mut Context,
    ) -> StreamStatus {
        if output
            .state
            .shared_data
            .pending_startover
            .swap(false, Ordering::SeqCst)
        {
            output.input.start_over_at(0);
        }
        output.input.step(dst, InputContext::new(context));

        if let Err(e) = output.state.shared_data.chunk_sender.try_send(*dst) {
            match e {
                TrySendError::Full(_) => println!("Output sound processor dropped a chunk"),
                TrySendError::Disconnected(_) => panic!("Idk what to do, maybe nothing?"),
            }
        }
        StreamStatus::Playing
    }
}

impl ProcessorState for OutputState {
    type Processor = Output;

    fn new(processor: &Self::Processor) -> Self {
        let host = cpal::default_host();
        // TODO: propagate these errors
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

        let mut chunk_index: usize = 0;
        let mut current_chunk: Option<SoundChunk> = None;

        let shared_data = Arc::clone(&processor.shared_data);

        let mut get_next_sample = move || {
            if current_chunk.is_none() || chunk_index >= CHUNK_SIZE {
                current_chunk = Some(
                    shared_data
                        .chunk_receiver
                        .lock()
                        .recv()
                        .unwrap_or_else(|_| SoundChunk::new()),
                );
                chunk_index = 0;
            }
            let c = current_chunk.as_ref().unwrap();
            let l = c.l[chunk_index];
            let r = c.r[chunk_index];
            chunk_index += 1;
            let l = if l.is_finite() {
                l.clamp(-1.0, 1.0)
            } else {
                0.0
            };
            let r = if r.is_finite() {
                r.clamp(-1.0, 1.0)
            } else {
                0.0
            };
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

        let barrier = Arc::new(Barrier::new(2));
        let barrier2 = Arc::clone(&barrier);

        // NOTE: Stream is not Send, using a dedicated thread as a workaround
        // See https://github.com/RustAudio/cpal/issues/818
        std::thread::spawn(move || {
            println!(
                "Requesting output audio stream with {} channels, a {} Hz sample rate, and a buffer size of {:?}",
                config.channels, config.sample_rate.0, config.buffer_size
            );

            let stream = device
                .build_output_stream(&config, data_callback, err_callback)
                .unwrap();
            stream.play().unwrap();
            barrier.wait();
            stream.pause().unwrap();
        });

        OutputState {
            shared_data: Arc::clone(&processor.shared_data),
            stream_end_barrier: barrier2,
        }
    }
}

impl WithObjectType for Output {
    const TYPE: ObjectType = ObjectType::new("output");
}

impl Stashable<StashingContext> for Output {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object(&self.input);
    }
}

impl UnstashableInplace<UnstashingContext<'_>> for Output {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext>,
    ) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.input)?;
        Ok(())
    }
}
