use crate::{
    core::{
        graph::graphobject::ObjectInitialization,
        sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    },
    objects::readwritewaveform::ReadWriteWaveform,
    ui_core::{
        expressionplot::PlotConfig, object_ui::ObjectUi, soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct ReadWriteWaveformUi {}

impl ObjectUi for ReadWriteWaveformUi {
    type GraphUi = SoundGraphUi;
    type HandleType = DynamicSoundProcessorHandle<ReadWriteWaveform>;
    type StateType = ();

    fn ui(
        &self,
        rww: DynamicSoundProcessorHandle<ReadWriteWaveform>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut (),
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&rww, "ReadWriteWaveform")
            .add_sound_input(rww.sound_input.id(), "input", sound_graph)
            .add_argument(rww.input_l.id(), "l")
            .add_argument(rww.input_r.id(), "r")
            .add_expression(rww.waveform.id(), "waveform", PlotConfig::new())
            .show(ui, ctx, graph_ui_state, sound_graph);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["readwritewaveform"]
    }

    fn make_properties(&self) -> () {
        ()
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: ObjectInitialization,
    ) -> Result<(), ()> {
        Ok(())
    }
}
