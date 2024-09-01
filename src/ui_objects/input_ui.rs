use eframe::egui;

use crate::{
    core::{
        graph::graphobject::ObjectInitialization,
        sound::{soundgraph::SoundGraph, soundprocessor::StaticSoundProcessorHandle},
        soundchunk::{SoundChunk, CHUNK_SIZE},
    },
    objects::input::Input,
    ui_core::{
        object_ui::ObjectUi, soundgraphui::SoundGraphUi, soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState, soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct InputUi {}

pub struct InputUiState {
    buffer_reader: spmcq::Reader<SoundChunk>,
    amplitude_history: Vec<f32>,
}

impl InputUi {
    fn update_amplitude_history(state: &mut InputUiState) {
        while let Some(chunk) = state.buffer_reader.read().value() {
            // TODO: display dropouts?
            let power_sum_l: f32 = chunk.l.iter().map(|s| s * s).sum();
            let power_sum_r: f32 = chunk.r.iter().map(|s| s * s).sum();
            let power_avg = (power_sum_l + power_sum_r) / (2.0 * CHUNK_SIZE as f32);
            let min_power: f32 = 1e-3;
            let max_power: f32 = 1.0;
            let log_min_power = min_power.ln();
            let log_max_power = max_power.ln();
            let power_clipped = power_avg.clamp(min_power, max_power);
            let t = (power_clipped.ln() - log_min_power) / (log_max_power - log_min_power);
            state.amplitude_history.pop();
            state.amplitude_history.insert(0, t);
        }
    }
}

impl ObjectUi for InputUi {
    type GraphUi = SoundGraphUi;
    type HandleType = StaticSoundProcessorHandle<Input>;
    type StateType = InputUiState;
    fn ui(
        &self,
        input: StaticSoundProcessorHandle<Input>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        state: &mut InputUiState,
        sound_graph: &mut SoundGraph,
    ) {
        Self::update_amplitude_history(state);

        // TODO: controls for choosing input device?
        // Would require changes to input
        ProcessorUi::new(&input, "Input").show_with(
            ui,
            ctx,
            graph_ui_state,
            sound_graph,
            |ui, _ui_state, _sound_graph| {
                let (_, rect) = ui.allocate_space(egui::vec2(100.0, 100.0));
                let painter = ui.painter();
                painter.rect_filled(rect, egui::Rounding::ZERO, egui::Color32::BLACK);

                let hist = &state.amplitude_history;

                let dy = rect.height() / hist.len() as f32;
                for (i, v) in hist.iter().enumerate() {
                    let y0 = i as f32 * dy;
                    let y1 = ((i + 1) as f32) * dy;
                    let w = v * rect.width();
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(rect.left(), rect.top() + y0),
                            egui::pos2(rect.left() + w, rect.top() + y1),
                        ),
                        egui::Rounding::ZERO,
                        egui::Color32::WHITE,
                    );
                }

                ui.ctx().request_repaint();
            },
        );
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["input"]
    }

    fn make_properties(&self) -> () {
        ()
    }

    fn make_ui_state(
        &self,
        handle: &Self::HandleType,
        _init: ObjectInitialization,
    ) -> Result<InputUiState, ()> {
        let mut amplitude_history = Vec::new();
        amplitude_history.resize(100, 0.0);
        Ok(InputUiState {
            buffer_reader: handle.get_buffer_reader(),
            amplitude_history,
        })
    }
}
