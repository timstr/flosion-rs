use crate::{
    core::{
        graph::{graphobject::ObjectHandle, objectfactory::ObjectFactory},
        number::numbergraph::NumberGraph,
        sound::soundgraph::SoundGraph,
    },
    ui_core::{
        graph_ui::GraphUi, numbergraphui::NumberGraphUi, object_ui::ObjectUi,
        soundgraphui::SoundGraphUi, ui_factory::UiFactory,
    },
};

use super::{
    audioclip_ui::AudioClipUi,
    dac_ui::DacUi,
    ensemble_ui::EnsembleUi,
    functions_ui::{
        AbsUi, AddUi, CeilUi, ConstantUi, CopysignUi, CosUi, CosineWaveUi, DivideUi, Exp10Ui,
        Exp2Ui, ExpUi, ExponentialApproachUi, FloorUi, FractUi, LerpUi, Log10Ui, Log2Ui, LogUi,
        MultiplyUi, NegateUi, PowUi, RoundUi, SawWaveUi, SignumUi, SinUi, SineWaveUi, SliderUi,
        SquareWaveUi, SubtractUi, TriangleWaveUi, TruncUi,
    },
    mixer_ui::MixerUi,
    recorder_ui::RecorderUi,
    resampler_ui::ResamplerUi,
    wavegenerator_ui::WaveGeneratorUi,
    whitenoise_ui::WhiteNoiseUi,
};

struct RegistrationHelper<'a, G: GraphUi> {
    object_factory: &'a mut ObjectFactory<G::Graph>,
    ui_factory: &'a mut UiFactory<G>,
}

impl<'a, G: GraphUi> RegistrationHelper<'a, G> {
    fn new(
        object_factory: &'a mut ObjectFactory<G::Graph>,
        ui_factory: &'a mut UiFactory<G>,
    ) -> RegistrationHelper<'a, G> {
        RegistrationHelper {
            object_factory,
            ui_factory,
        }
    }

    fn register<T: ObjectUi<GraphUi = G>>(&mut self) {
        self.object_factory
            .register::<<T::HandleType as ObjectHandle<G::Graph>>::ObjectType>();
        self.ui_factory.register::<T>();
    }
}

pub fn all_sound_graph_objects() -> (ObjectFactory<SoundGraph>, UiFactory<SoundGraphUi>) {
    let mut object_factory = ObjectFactory::new_empty();
    let mut ui_factory = UiFactory::new_empty();

    let mut helper = RegistrationHelper::new(&mut object_factory, &mut ui_factory);

    // Static sound processors
    helper.register::<DacUi>();
    // helper.register_static_sound_processor::<KeyboardUi>();
    helper.register::<RecorderUi>();

    // Dynamic sound processors
    // helper.register_dynamic_sound_processor::<ADSRUi>();
    helper.register::<AudioClipUi>();
    helper.register::<EnsembleUi>();
    // helper.register_dynamic_sound_processor::<MelodyUi>();
    helper.register::<MixerUi>();
    helper.register::<ResamplerUi>();
    helper.register::<WaveGeneratorUi>();
    helper.register::<WhiteNoiseUi>();

    (object_factory, ui_factory)
}

pub fn all_number_graph_objects() -> (ObjectFactory<NumberGraph>, UiFactory<NumberGraphUi>) {
    let mut object_factory = ObjectFactory::new_empty();
    let mut ui_factory = UiFactory::new_empty();

    let mut helper = RegistrationHelper::new(&mut object_factory, &mut ui_factory);

    helper.register::<ConstantUi>();
    helper.register::<SliderUi>();

    helper.register::<ExponentialApproachUi>();

    helper.register::<NegateUi>();
    helper.register::<FloorUi>();
    helper.register::<CeilUi>();
    helper.register::<RoundUi>();
    helper.register::<TruncUi>();
    helper.register::<FractUi>();
    helper.register::<AbsUi>();
    helper.register::<SignumUi>();
    helper.register::<ExpUi>();
    helper.register::<Exp2Ui>();
    helper.register::<Exp10Ui>();
    helper.register::<LogUi>();
    helper.register::<Log2Ui>();
    helper.register::<Log10Ui>();
    // helper.register::<CbrtUi>();
    helper.register::<SinUi>();
    helper.register::<CosUi>();
    // helper.register::<TanUi>();
    // helper.register::<AsinUi>();
    // helper.register::<AcosUi>();
    // helper.register::<AtanUi>();
    // helper.register::<SinhUi>();
    // helper.register::<CoshUi>();
    // helper.register::<TanhUi>();
    // helper.register::<AsinhUi>();
    // helper.register::<AcoshUi>();
    // helper.register::<AtanhUi>();

    helper.register::<SineWaveUi>();
    helper.register::<CosineWaveUi>();
    helper.register::<SquareWaveUi>();
    helper.register::<SawWaveUi>();
    helper.register::<TriangleWaveUi>();

    helper.register::<AddUi>();
    helper.register::<SubtractUi>();
    helper.register::<MultiplyUi>();
    helper.register::<DivideUi>();
    // helper.register::<HypotUi>();
    helper.register::<CopysignUi>();
    helper.register::<PowUi>();
    // helper.register::<Atan2Ui>();

    helper.register::<LerpUi>();

    (object_factory, ui_factory)
}
