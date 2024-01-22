use eframe::egui;
use serialization::{Deserializer, Serializable, Serializer};

use crate::{
    core::{
        sound::{soundgraph::SoundGraph, soundprocessor::StaticSoundProcessorHandle},
        soundchunk::{SoundChunk, CHUNK_SIZE},
    },
    objects::input::Input,
    ui_core::{
        graph_ui::ObjectUiState,
        object_ui::{Color, ObjectUi, UiInitialization},
        soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState,
        soundobjectuistate::SoundObjectUiData,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct InputUi {}

pub struct InputUiState {
    buffer_reader: ringbuffer::Reader<SoundChunk>,
}

// TODO: this doesn't make sense
impl Serializable for InputUiState {
    fn serialize(&self, serializer: &mut Serializer) {}

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, ()> {
        Err(())
    }
}

impl ObjectUiState for InputUiState {}

impl ObjectUi for InputUi {
    type GraphUi = SoundGraphUi;
    type HandleType = StaticSoundProcessorHandle<Input>;
    type StateType = InputUiState;
    fn ui(
        &self,
        input: StaticSoundProcessorHandle<Input>,
        ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &mut SoundGraphUiContext,
        data: SoundObjectUiData<Self::StateType>,
        sound_graph: &mut SoundGraph,
    ) {
        // TODO: controls for choosing input device?
        // Would require changes to input
        ProcessorUi::new(&input, "Input", data.color).show_with(
            ui,
            ctx,
            ui_state,
            sound_graph,
            |ui, _ui_state, _sound_graph| {
                // TODO: keep this reader between draw calls
                let reader = &mut data.state.buffer_reader;
                let color = match reader.read().value() {
                    Some(chunk) => {
                        let power_sum_l: f32 = chunk.l.iter().map(|s| s * s).sum();
                        let power_sum_r: f32 = chunk.r.iter().map(|s| s * s).sum();
                        let power_avg = (power_sum_l + power_sum_r) / (2.0 * CHUNK_SIZE as f32);
                        let min_power: f32 = 1e-3;
                        let max_power: f32 = 1.0;
                        let log_min_power = min_power.ln();
                        let log_max_power = max_power.ln();
                        let power_clipped = power_avg.clamp(min_power, max_power);
                        let t =
                            (power_clipped.ln() - log_min_power) / (log_max_power - log_min_power);
                        let v = (t * 255.0).round() as u8;
                        egui::Color32::from_rgb(v, v, 0)
                    }
                    None => egui::Color32::YELLOW,
                };

                let (_, rect) = ui.allocate_space(egui::vec2(250.0, 50.0));
                let painter = ui.painter();
                painter.rect_filled(rect, egui::Rounding::none(), color);
                ui.ctx().request_repaint();
            },
        );
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["input"]
    }

    fn make_ui_state(
        &self,
        handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (Self::StateType, Color) {
        (
            InputUiState {
                buffer_reader: handle.get_buffer_reader(),
            },
            Color::default(),
        )
    }
}
