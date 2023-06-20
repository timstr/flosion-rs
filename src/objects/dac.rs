use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{sync_channel, SyncSender, TrySendError},
    Arc, Barrier,
};

use crate::core::{
    engine::nodegen::NodeGen,
    resample::resample_interleave,
    samplefrequency::SAMPLE_FREQUENCY,
    sound::{
        context::Context,
        graphobject::{ObjectInitialization, ObjectType, WithObjectType},
        soundinput::InputOptions,
        soundinputtypes::{SingleInput, SingleInputNode},
        soundprocessor::StaticSoundProcessor,
        soundprocessortools::SoundProcessorTools,
    },
    soundchunk::{SoundChunk, CHUNK_SIZE},
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

impl Dac {
    pub fn reset(&self) {
        self.shared_data.pending_reset.store(true, Ordering::SeqCst);
    }
}

impl StaticSoundProcessor for Dac {
    type SoundInputType = SingleInput;
    type NumberInputType<'ctx> = ();

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
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

        Ok(Dac {
            input: SingleInput::new(InputOptions::Synchronous, &mut tools),
            shared_data,
        })
    }

    fn get_sound_input(&self) -> &SingleInput {
        &self.input
    }

    fn make_number_inputs<'a, 'ctx>(
        &self,
        _nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self::NumberInputType<'ctx> {
        ()
    }

    fn process_audio(
        &self,
        sound_input: &mut SingleInputNode,
        _number_input: &(),
        _dst: &mut SoundChunk,
        ctx: Context,
    ) {
        if sound_input.timing().needs_reset()
            || self.shared_data.pending_reset.swap(false, Ordering::SeqCst)
        {
            sound_input.reset(0);
        }
        let mut ch = SoundChunk::new();
        sound_input.step(self, &mut ch, &ctx);

        if let Err(e) = self.shared_data.chunk_sender.try_send(ch) {
            match e {
                TrySendError::Full(_) => println!("Dac dropped a chunk"),
                TrySendError::Disconnected(_) => panic!("Idk what to do, maybe nothing?"),
            }
        }
    }
}

impl WithObjectType for Dac {
    const TYPE: ObjectType = ObjectType::new("dac");
}
