use crate::{
    core::{
        graphobject::ObjectHandle,
        numbersource::PureNumberSource,
        object_factory::ObjectFactory,
        soundprocessor::{DynamicSoundProcessor, StaticSoundProcessor},
    },
    ui_core::{object_ui::ObjectUi, ui_factory::UiFactory},
};

use super::{
    adsr_ui::ADSRUi, audioclip_ui::AudioClipUi, dac_ui::DacUi, ensemble_ui::EnsembleUi,
    functions_ui::*, keyboard_ui::KeyboardUi, melody_ui::MelodyUi, mixer_ui::MixerUi,
    recorder_ui::RecorderUi, resampler_ui::ResamplerUi, wavegenerator_ui::WaveGeneratorUi,
    whitenoise_ui::WhiteNoiseUi,
};

struct RegistrationHelper<'a> {
    object_factory: &'a mut ObjectFactory,
    ui_factory: &'a mut UiFactory,
}

impl<'a> RegistrationHelper<'a> {
    fn new(
        object_factory: &'a mut ObjectFactory,
        ui_factory: &'a mut UiFactory,
    ) -> RegistrationHelper<'a> {
        RegistrationHelper {
            object_factory,
            ui_factory,
        }
    }

    fn register_static_sound_processor<T: ObjectUi>(&mut self)
    where
        <<T as ObjectUi>::HandleType as ObjectHandle>::Type: StaticSoundProcessor,
    {
        self.object_factory
            .register_static_sound_processor::<<<T as ObjectUi>::HandleType as ObjectHandle>::Type>(
            );
        self.ui_factory.register_static_sound_processor::<T>();
    }

    fn register_dynamic_sound_processor<T: ObjectUi>(&mut self)
    where
        <<T as ObjectUi>::HandleType as ObjectHandle>::Type: DynamicSoundProcessor,
    {
        self.object_factory
            .register_dynamic_sound_processor::<<<T as ObjectUi>::HandleType as ObjectHandle>::Type>();
        self.ui_factory.register_dynamic_sound_processor::<T>();
    }

    fn register_number_source<T: ObjectUi>(&mut self)
    where
        <<T as ObjectUi>::HandleType as ObjectHandle>::Type: PureNumberSource,
    {
        self.object_factory
            .register_number_source::<<<T as ObjectUi>::HandleType as ObjectHandle>::Type>();
        self.ui_factory.register_number_source::<T>();
    }
}

pub fn all_objects() -> (ObjectFactory, UiFactory) {
    let mut object_factory = ObjectFactory::new_empty();
    let mut ui_factory = UiFactory::new_empty();

    let mut helper = RegistrationHelper::new(&mut object_factory, &mut ui_factory);

    // Static sound processors
    helper.register_static_sound_processor::<DacUi>();
    helper.register_static_sound_processor::<KeyboardUi>();
    helper.register_static_sound_processor::<RecorderUi>();

    // Dynamicic sound processors
    helper.register_dynamic_sound_processor::<ADSRUi>();
    helper.register_dynamic_sound_processor::<AudioClipUi>();
    helper.register_dynamic_sound_processor::<EnsembleUi>();
    helper.register_dynamic_sound_processor::<MelodyUi>();
    helper.register_dynamic_sound_processor::<MixerUi>();
    helper.register_dynamic_sound_processor::<ResamplerUi>();
    helper.register_dynamic_sound_processor::<WaveGeneratorUi>();
    helper.register_dynamic_sound_processor::<WhiteNoiseUi>();

    // Pure number sources
    helper.register_number_source::<ConstantUi>();

    helper.register_number_source::<NegateUi>();
    helper.register_number_source::<FloorUi>();
    helper.register_number_source::<CeilUi>();
    helper.register_number_source::<RoundUi>();
    helper.register_number_source::<TruncUi>();
    helper.register_number_source::<FractUi>();
    helper.register_number_source::<AbsUi>();
    // helper.register_number_source::<SignumUi>();
    helper.register_number_source::<ExpUi>();
    helper.register_number_source::<Exp2Ui>();
    // helper.register_number_source::<Exp10Ui>();
    helper.register_number_source::<LogUi>();
    helper.register_number_source::<Log2Ui>();
    helper.register_number_source::<Log10Ui>();
    // helper.register_number_source::<CbrtUi>();
    helper.register_number_source::<SinUi>();
    helper.register_number_source::<CosUi>();
    // helper.register_number_source::<TanUi>();
    // helper.register_number_source::<AsinUi>();
    // helper.register_number_source::<AcosUi>();
    // helper.register_number_source::<AtanUi>();
    // helper.register_number_source::<SinhUi>();
    // helper.register_number_source::<CoshUi>();
    // helper.register_number_source::<TanhUi>();
    // helper.register_number_source::<AsinhUi>();
    // helper.register_number_source::<AcoshUi>();
    // helper.register_number_source::<AtanhUi>();

    helper.register_number_source::<SineWaveUi>();
    // helper.register_number_source::<CosineWaveUi>();
    // helper.register_number_source::<SquareWaveUi>();
    // helper.register_number_source::<SawWaveUi>();
    // helper.register_number_source::<TriangleWaveUi>();

    helper.register_number_source::<AddUi>();
    helper.register_number_source::<SubtractUi>();
    helper.register_number_source::<MultiplyUi>();
    helper.register_number_source::<DivideUi>();
    // helper.register_number_source::<HypotUi>();
    // helper.register_number_source::<CopysignUi>();
    helper.register_number_source::<PowUi>();
    // helper.register_number_source::<Atan2Ui>();

    (object_factory, ui_factory)
}
