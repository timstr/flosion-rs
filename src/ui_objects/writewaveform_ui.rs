use crate::{
    core::{
        graph::graphobject::ObjectInitialization,
        sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    },
    objects::writewaveform::WriteWaveform,
    ui_core::{
        expressionplot::PlotConfig, object_ui::ObjectUi, soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
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
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut (),
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&ww, "WriteWaveform")
            .add_expression(ww.waveform.id(), "waveform", PlotConfig::new())
            .show(ui, ctx, graph_ui_state, sound_graph);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["writewaveform"]
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: ObjectInitialization,
    ) -> Result<(), ()> {
        Ok(())
    }
}
