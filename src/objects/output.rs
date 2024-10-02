use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{sync_channel, SyncSender, TrySendError},
    Arc, Barrier,
};

use crate::{
    core::{
        engine::soundgraphcompiler::SoundGraphCompiler,
        objecttype::{ObjectType, WithObjectType},
        resample::resample_interleave,
        samplefrequency::SAMPLE_FREQUENCY,
        sound::{
            context::{Context, LocalArrayList},
            expression::ProcessorExpression,
            soundinput::InputOptions,
            soundinputtypes::{SingleInput, SingleInputNode},
            soundprocessor::{
                SoundProcessorId, StateAndTiming, StreamStatus, WhateverSoundProcessor,
            },
            soundprocessortools::SoundProcessorTools,
            state::State,
        },
        soundchunk::{SoundChunk, CHUNK_SIZE},
    },
    ui_core::arguments::ParsedArguments,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleRate, StreamConfig, StreamError,
};

pub struct OutputData {
    stream_end_barrier: Barrier,
    pending_startover: AtomicBool,
    chunk_sender: SyncSender<SoundChunk>,
}

// TODO: rename to e.g. "SoundOut", "Output" is too vague and overloaded
// AudioOut?
pub struct Output {
    pub input: SingleInput,
    shared_data: Arc<OutputData>,
}

impl Output {
    pub fn start_over(&self) {
        self.shared_data
            .pending_startover
            .store(true, Ordering::SeqCst);
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        self.shared_data.stream_end_barrier.wait();
    }
}

pub struct OutputState {
    shared_data: Arc<OutputData>,
}

impl State for OutputState {
    fn start_over(&mut self) {
        // ???
    }
}

impl WhateverSoundProcessor for Output {
    type SoundInputType = SingleInput;
    type Expressions<'ctx> = ();
    type StateType = OutputState;

    fn new(mut tools: SoundProcessorTools, _args: &ParsedArguments) -> Result<Self, ()> {
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

        let (tx, rx) = sync_channel::<SoundChunk>(0);

        let shared_data = Arc::new(OutputData {
            chunk_sender: tx,
            pending_startover: AtomicBool::new(false),
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

        let shared_data_also = Arc::clone(&shared_data);

        // NOTE: Stream is not Send, using a dedicated thread as a workaround
        std::thread::spawn(move || {
            println!(
                "Requesting output audio stream with {} channels, a {} Hz sample rate, and a buffer size of {:?}",
                config.channels, config.sample_rate.0, config.buffer_size
            );

            let stream = device
                .build_output_stream(&config, data_callback, err_callback)
                .unwrap();
            stream.play().unwrap();
            shared_data_also.stream_end_barrier.wait();
            stream.pause().unwrap();
        });

        Ok(Output {
            input: SingleInput::new(InputOptions::Synchronous, &mut tools),
            shared_data,
        })
    }

    fn is_static(&self) -> bool {
        true
    }

    fn get_sound_input(&self) -> &SingleInput {
        &self.input
    }

    fn visit_expressions<'a>(&self, _f: Box<dyn 'a + FnMut(&ProcessorExpression)>) {}
    fn visit_expressions_mut<'a>(&mut self, _f: Box<dyn 'a + FnMut(&mut ProcessorExpression)>) {}

    fn compile_expressions<'a, 'ctx>(
        &self,
        _processor_id: SoundProcessorId,
        _compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        ()
    }

    fn make_state(&self) -> Self::StateType {
        OutputState {
            shared_data: Arc::clone(&self.shared_data),
        }
    }

    fn process_audio(
        state: &mut StateAndTiming<Self::StateType>,
        sound_input: &mut SingleInputNode,
        _expressions: &mut (),
        _dst: &mut SoundChunk,
        ctx: Context,
    ) -> StreamStatus {
        if state
            .shared_data
            .pending_startover
            .swap(false, Ordering::SeqCst)
        {
            sound_input.start_over(0);
        }
        let mut ch = SoundChunk::new();
        sound_input.step(state, &mut ch, &ctx, LocalArrayList::new());

        if let Err(e) = state.shared_data.chunk_sender.try_send(ch) {
            match e {
                TrySendError::Full(_) => println!("Output sound processor dropped a chunk"),
                TrySendError::Disconnected(_) => panic!("Idk what to do, maybe nothing?"),
            }
        }
        StreamStatus::Playing
    }
}

impl WithObjectType for Output {
    const TYPE: ObjectType = ObjectType::new("output");
}
