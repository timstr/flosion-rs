use crate::{
    core::{
        expression::expressionobject::{ExpressionObjectFactory, ExpressionObjectHandle},
        sound::soundobject::{SoundObjectFactory, SoundObjectHandle},
    },
    ui_core::{
        expressionobjectui::{ExpressionObjectUi, ExpressionObjectUiFactory},
        soundobjectui::{SoundObjectUi, SoundObjectUiFactory},
    },
};

use super::{
    adsr_ui::ADSRUi,
    audioclip_ui::AudioClipUi,
    // definitions_ui::DefinitionsUi,
    // ensemble_ui::EnsembleUi,
    // input_ui::InputUi,
    // keyboard_ui::KeyboardUi,
    // mixer_ui::MixerUi,
    // oscilloscope_ui::OscilloscopeUi,
    // output_ui::OutputUi,
    pure_function_uis::{
        AbsUi, AddUi, CeilUi, ConstantUi, CopysignUi, CosUi, CosineWaveUi, DivideUi, Exp10Ui,
        Exp2Ui, ExpUi, FloorUi, FractUi, LerpUi, Log10Ui, Log2Ui, LogUi, MultiplyUi, NegateUi,
        PowUi, RoundUi, SawWaveUi, SignumUi, SinUi, SineWaveUi, SliderUi, SquareWaveUi, SubtractUi,
        TriangleWaveUi, TruncUi,
    },
    // readwritewaveform_ui::ReadWriteWaveformUi,
    // recorder_ui::RecorderUi,
    // resampler_ui::ResamplerUi,
    // sampler1d_ui::Sampler1dUi,
    // scatter_ui::ScatterUi,
    // stateful_function_uis::{
    //     ExponentialApproachUi, IntegratorUi, LinearApproachUi, WrappingIntegratorUi,
    // },
    // wavegenerator_ui::WaveGeneratorUi,
    // whitenoise_ui::WhiteNoiseUi,
    // writewaveform_ui::WriteWaveformUi,
};

struct ExpressionObjectRegistrationHelper<'a> {
    object_factory: &'a mut ExpressionObjectFactory,
    ui_factory: &'a mut ExpressionObjectUiFactory,
}

impl<'a> ExpressionObjectRegistrationHelper<'a> {
    fn new(
        object_factory: &'a mut ExpressionObjectFactory,
        ui_factory: &'a mut ExpressionObjectUiFactory,
    ) -> Self {
        ExpressionObjectRegistrationHelper {
            object_factory,
            ui_factory,
        }
    }

    fn register<T: 'static + ExpressionObjectUi>(&mut self) {
        // Yikes
        self.object_factory
            .register::<<<T as ExpressionObjectUi>::HandleType as ExpressionObjectHandle>::ObjectType>();
        self.ui_factory.register::<T>();
    }
}
struct SoundObjectRegistrationHelper<'a> {
    object_factory: &'a mut SoundObjectFactory,
    ui_factory: &'a mut SoundObjectUiFactory,
}

impl<'a> SoundObjectRegistrationHelper<'a> {
    fn new(
        object_factory: &'a mut SoundObjectFactory,
        ui_factory: &'a mut SoundObjectUiFactory,
    ) -> Self {
        SoundObjectRegistrationHelper {
            object_factory,
            ui_factory,
        }
    }

    fn register<T: 'static + SoundObjectUi>(&mut self) {
        self.object_factory
            .register::<<T::HandleType as SoundObjectHandle>::ObjectType>();
        self.ui_factory.register::<T>();
    }
}

pub(crate) fn all_sound_graph_objects() -> (SoundObjectFactory, SoundObjectUiFactory) {
    let mut object_factory = SoundObjectFactory::new_empty();
    let mut ui_factory = SoundObjectUiFactory::new_empty();

    let mut helper = SoundObjectRegistrationHelper::new(&mut object_factory, &mut ui_factory);

    // Static sound processors
    // helper.register::<OutputUi>();
    // helper.register::<KeyboardUi>();
    // helper.register::<RecorderUi>();
    // helper.register::<OscilloscopeUi>();

    // Dynamic sound processors
    helper.register::<ADSRUi>();
    helper.register::<AudioClipUi>();
    // helper.register::<DefinitionsUi>();
    // helper.register::<EnsembleUi>();
    // helper.register::<InputUi>();
    // helper.register_dynamic_sound_processor::<MelodyUi>();
    // helper.register::<MixerUi>();
    // helper.register::<ReadWriteWaveformUi>();
    // helper.register::<ResamplerUi>();
    // helper.register::<ScatterUi>();
    // helper.register::<WaveGeneratorUi>();
    // helper.register::<WhiteNoiseUi>();
    // helper.register::<WriteWaveformUi>();

    (object_factory, ui_factory)
}

pub(crate) fn all_expression_graph_objects() -> (ExpressionObjectFactory, ExpressionObjectUiFactory)
{
    let mut object_factory = ExpressionObjectFactory::new_empty();
    let mut ui_factory = ExpressionObjectUiFactory::new_empty();

    let mut helper = ExpressionObjectRegistrationHelper::new(&mut object_factory, &mut ui_factory);

    helper.register::<ConstantUi>();
    helper.register::<SliderUi>();

    // helper.register::<LinearApproachUi>();
    // helper.register::<ExponentialApproachUi>();
    // helper.register::<IntegratorUi>();
    // helper.register::<WrappingIntegratorUi>();
    // helper.register::<Sampler1dUi>();

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
