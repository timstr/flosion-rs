use std::sync::{Arc, Barrier};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, SampleRate, StreamConfig,
};
use eframe::egui::mutex::Mutex;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};
use spmcq::ReadResult;

use crate::{
    core::{
        engine::soundgraphcompiler::SoundGraphCompiler,
        objecttype::{ObjectType, WithObjectType},
        samplefrequency::SAMPLE_FREQUENCY,
        sound::{
            context::Context,
            soundprocessor::{
                ProcessorComponent, ProcessorComponentVisitor, ProcessorComponentVisitorMut,
                SoundProcessor, SoundProcessorId, StartOver, StreamStatus,
            },
        },
        soundchunk::{SoundChunk, CHUNK_SIZE},
    },
    ui_core::arguments::ParsedArguments,
};

// TODO: rename to something less vague
// AudioIn?

pub struct Input {
    chunk_reader: spmcq::Reader<SoundChunk>,
    // TODO: improve this. It is only accessed by the
    // audio thread but the current compilation interface
    // doesn't lend itself to static processors in this way
    chunk_writer: Arc<Mutex<spmcq::Writer<SoundChunk>>>,
}

impl Input {
    pub fn get_buffer_reader(&self) -> spmcq::Reader<SoundChunk> {
        self.chunk_reader.clone()
    }
}

pub struct CompiledInput {
    chunk_receiver: spmcq::Reader<SoundChunk>,
    stream_end_barrier: Arc<Barrier>,
}

impl SoundProcessor for Input {
    fn new(_args: &ParsedArguments) -> Input {
        let (reader, writer) = spmcq::ring_buffer::<SoundChunk>(8);

        Input {
            chunk_reader: reader,
            chunk_writer: Arc::new(Mutex::new(writer)),
        }
    }

    fn is_static(&self) -> bool {
        true
    }

    fn process_audio(
        input: &mut CompiledInput,
        dst: &mut SoundChunk,
        _context: &mut Context,
    ) -> StreamStatus {
        let chunk = match input.chunk_receiver.read() {
            ReadResult::Ok(ch) => ch,
            ReadResult::Dropout(ch) => {
                println!("WARNING: Input dropout");
                ch
            }
            ReadResult::Empty => return StreamStatus::Playing,
        };
        *dst = chunk;
        StreamStatus::Playing
    }
}

impl ProcessorComponent for Input {
    type CompiledType<'ctx> = CompiledInput;

    fn visit<'a>(&self, _visitor: &'a mut dyn ProcessorComponentVisitor) {}

    fn visit_mut<'a>(&mut self, _visitor: &'a mut dyn ProcessorComponentVisitorMut) {}

    fn compile<'ctx>(
        &self,
        _processor_id: SoundProcessorId,
        _compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .expect("No input device available"); // TODO: error handling

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

        config.channels = 2;

        let mut current_chunk = SoundChunk::new();
        let mut chunk_cursor: usize = 0;

        let chunk_writer = Arc::clone(&self.chunk_writer);

        let data_callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            for sample in data.chunks_exact(2) {
                current_chunk.l[chunk_cursor] = sample[0];
                current_chunk.r[chunk_cursor] = sample[1];
                chunk_cursor += 1;
                if chunk_cursor == CHUNK_SIZE {
                    chunk_cursor = 0;
                    // TODO: remove locking here
                    chunk_writer.lock().write(current_chunk);
                }
            }
        };

        let barrier = Arc::new(Barrier::new(2));
        let barrier2 = Arc::clone(&barrier);

        // NOTE: Stream is not Send, using a dedicated thread as a workaround
        // See https://github.com/RustAudio/cpal/issues/818
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

        CompiledInput {
            chunk_receiver: self.chunk_reader.clone(),
            stream_end_barrier: barrier,
        }
    }
}

impl StartOver for CompiledInput {
    fn start_over(&mut self) {
        self.chunk_receiver.skip_ahead();
    }
}

impl Drop for CompiledInput {
    fn drop(&mut self) {
        self.stream_end_barrier.wait();
    }
}

impl WithObjectType for Input {
    const TYPE: ObjectType = ObjectType::new("input");
}

impl Stashable for Input {
    fn stash(&self, _stasher: &mut Stasher) {
        // TODO: once different options are supported (e.g. which device?),
        // stash those
    }
}

impl UnstashableInplace for Input {
    fn unstash_inplace(&mut self, _unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        Ok(())
    }
}
