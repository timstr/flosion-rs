use eframe::egui;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::{
    core::{
        sound::soundprocessor::SoundProcessorWithId,
        soundchunk::{SoundChunk, CHUNK_SIZE},
    },
    objects::input::Input,
    ui_core::{
        arguments::ParsedArguments, soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState, soundobjectui::SoundObjectUi,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct InputUi {}

pub struct InputUiState {
    buffer_reader: spmcq::Reader<SoundChunk>,
    amplitude_history: Vec<f32>,
}

impl Stashable for InputUiState {
    fn stash(&self, _stasher: &mut Stasher) {}
}

impl UnstashableInplace for InputUiState {
    fn unstash_inplace(&mut self, _unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        Ok(())
    }
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

impl SoundObjectUi for InputUi {
    type ObjectType = SoundProcessorWithId<Input>;
    type StateType = InputUiState;
    fn ui(
        &self,
        input: &mut SoundProcessorWithId<Input>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        state: &mut InputUiState,
    ) {
        Self::update_amplitude_history(state);

        // TODO: controls for choosing input device?
        // Would require changes to input
        ProcessorUi::new("Input").show_with(
            input,
            ui,
            ctx,
            graph_ui_state,
            |_input, ui, _ui_state| {
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
        audioclip: &Self::ObjectType,
        _args: &ParsedArguments,
    ) -> Result<InputUiState, ()> {
        let mut amplitude_history = Vec::new();
        amplitude_history.resize(100, 0.0);
        Ok(InputUiState {
            buffer_reader: audioclip.get_buffer_reader(),
            amplitude_history,
        })
    }
}
