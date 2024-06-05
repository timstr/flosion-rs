use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    objects::writewaveform::WriteWaveform,
    ui_core::{
        expressionplot::PlotConfig,
        object_ui::{Color, ObjectUi, UiInitialization},
        soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState,
        soundobjectuistate::SoundObjectUiData,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct WriteWaveformUi {}

impl ObjectUi for WriteWaveformUi {
    type GraphUi = SoundGraphUi;
    type HandleType = DynamicSoundProcessorHandle<WriteWaveform>;
    type StateType = ();

    fn ui(
        &self,
        ww: DynamicSoundProcessorHandle<WriteWaveform>,
        ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        data: SoundObjectUiData<()>,
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&ww, "WriteWaveform", data.color)
            .add_expression(ww.waveform.id(), "waveform", PlotConfig::new())
            .show(ui, ctx, ui_state, sound_graph);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["writewaveform"]
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (Self::StateType, Color) {
        ((), Color::default())
    }
}
