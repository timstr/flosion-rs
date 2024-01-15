use cpal::{
    traits::{DeviceTrait, HostTrait},
    SampleRate, StreamConfig,
};

use crate::core::{
    engine::nodegen::NodeGen,
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    samplefrequency::SAMPLE_FREQUENCY,
    sound::{
        context::Context,
        soundprocessor::{ProcessorTiming, StaticSoundProcessor, StaticSoundProcessorWithId},
        soundprocessortools::SoundProcessorTools,
    },
    soundchunk::SoundChunk,
};

pub struct Input {}

impl StaticSoundProcessor for Input {
    type SoundInputType = ();
    type NumberInputType<'ctx> = ();

    fn new(_tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| println!("No input device available"))?;

        let mut supported_configs_range = device
            .supported_input_configs()
            .expect("error while querying input configs");
        let supported_config = supported_configs_range
            .next()
            .expect("No supported input config:?")
            .with_sample_rate(SampleRate(SAMPLE_FREQUENCY as u32));
        let config = supported_config.into();
        let stream = device.build_input_stream(
            &config,
            |data, _: &cpal::InputCallbackInfo| {
                // TODO
                todo!()
            },
            |err| {
                // TODO
                todo!()
            },
        );

        Ok(Input {})
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
        timing: &ProcessorTiming,
        sound_inputs: &mut (),
        number_inputs: &mut (),
        dst: &mut SoundChunk,
        context: Context,
    ) {
        todo!()
    }
}

impl WithObjectType for Input {
    const TYPE: ObjectType = ObjectType::new("input");
}
