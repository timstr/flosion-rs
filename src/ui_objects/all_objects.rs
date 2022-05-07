use super::mixer_ui::MixerUi;
use super::object_factory::ObjectFactory;

use super::{
    audioclip_ui::AudioClipUi,
    dac_ui::DacUi,
    functions_ui::{
        AddUi, ConstantUi, DivideUi, MultiplyUi, NegateUi, SineUi, SubtractUi, UnitSineUi,
    },
    keyboard_ui::KeyboardUi,
    recorder_ui::RecorderUi,
    wavegenerator_ui::WaveGeneratorUi,
    whitenoise_ui::WhiteNoiseUi,
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
    all_uis.register_number_source::<AddUi>();
    all_uis.register_number_source::<SubtractUi>();
    all_uis.register_number_source::<MultiplyUi>();
    all_uis.register_number_source::<DivideUi>();
    all_uis.register_number_source::<SineUi>();
    all_uis.register_number_source::<UnitSineUi>();

    all_uis
}
