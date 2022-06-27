use super::mixer_ui::MixerUi;
use super::object_factory::ObjectFactory;

use super::{
    audioclip_ui::AudioClipUi, dac_ui::DacUi, functions_ui::*, keyboard_ui::KeyboardUi,
    recorder_ui::RecorderUi, wavegenerator_ui::WaveGeneratorUi, whitenoise_ui::WhiteNoiseUi,
};

pub fn all_objects() -> ObjectFactory {
    let mut all_uis = ObjectFactory::new_empty();

    // Static sound processors
    all_uis.register_static_sound_processor::<DacUi>();
    all_uis.register_static_sound_processor::<KeyboardUi>();
    all_uis.register_static_sound_processor::<RecorderUi>();

    // Dynamic sound processors
    all_uis.register_dynamic_sound_processor::<WaveGeneratorUi>();
    all_uis.register_dynamic_sound_processor::<WhiteNoiseUi>();
    all_uis.register_dynamic_sound_processor::<AudioClipUi>();
    all_uis.register_dynamic_sound_processor::<MixerUi>();

    // Pure number sources
    all_uis.register_number_source::<ConstantUi>();

    all_uis.register_number_source::<NegateUi>();
    all_uis.register_number_source::<FloorUi>();
    all_uis.register_number_source::<CeilUi>();
    all_uis.register_number_source::<RoundUi>();
    all_uis.register_number_source::<TruncUi>();
    all_uis.register_number_source::<FractUi>();
    all_uis.register_number_source::<AbsUi>();
    all_uis.register_number_source::<SignumUi>();
    all_uis.register_number_source::<ExpUi>();
    all_uis.register_number_source::<Exp2Ui>();
    all_uis.register_number_source::<Exp10Ui>();
    all_uis.register_number_source::<LogUi>();
    all_uis.register_number_source::<Log2Ui>();
    all_uis.register_number_source::<Log10Ui>();
    all_uis.register_number_source::<CbrtUi>();
    all_uis.register_number_source::<SinUi>();
    all_uis.register_number_source::<USinUi>();
    all_uis.register_number_source::<CosUi>();
    all_uis.register_number_source::<UCosUi>();
    all_uis.register_number_source::<TanUi>();
    all_uis.register_number_source::<AsinUi>();
    all_uis.register_number_source::<AcosUi>();
    all_uis.register_number_source::<AtanUi>();
    all_uis.register_number_source::<SinhUi>();
    all_uis.register_number_source::<CoshUi>();
    all_uis.register_number_source::<TanhUi>();
    all_uis.register_number_source::<AsinhUi>();
    all_uis.register_number_source::<AcoshUi>();
    all_uis.register_number_source::<AtanhUi>();

    all_uis.register_number_source::<AddUi>();
    all_uis.register_number_source::<SubtractUi>();
    all_uis.register_number_source::<MultiplyUi>();
    all_uis.register_number_source::<DivideUi>();
    all_uis.register_number_source::<HypotUi>();
    all_uis.register_number_source::<CopysignUi>();
    all_uis.register_number_source::<PowUi>();
    all_uis.register_number_source::<Atan2Ui>();

    all_uis
}
