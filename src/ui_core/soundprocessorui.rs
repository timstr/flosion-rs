use eframe::egui;

use crate::core::sound::{
    expression::SoundExpressionId,
    expressionargument::SoundExpressionArgumentId,
    soundgraph::SoundGraph,
    soundinput::SoundInputId,
    soundprocessor::{ProcessorHandle, SoundProcessorId},
};

use super::{
    expressionplot::PlotConfig, interactions::draganddrop::DragDropSubject,
    soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
};

pub struct ProcessorUi {
    processor_id: SoundProcessorId,
    label: &'static str,
    color: egui::Color32,
    expressions: Vec<(SoundExpressionId, String, PlotConfig)>,
    arguments: Vec<(SoundExpressionArgumentId, String)>,
    sound_inputs: Vec<(SoundInputId, String, SoundExpressionArgumentId)>,
}

#[derive(Clone, Copy)]
struct ProcessorUiProps {
    origin: egui::Pos2,
}

impl ProcessorUi {
    pub fn new<T: ProcessorHandle>(
        handle: &T,
        label: &'static str,
        color: egui::Color32,
    ) -> ProcessorUi {
        let mut arguments = Vec::new();
        arguments.push((handle.time_argument(), "time".to_string()));
        ProcessorUi {
            processor_id: handle.id(),
            label,
            color,
            expressions: Vec::new(),
            arguments,
            sound_inputs: Vec::new(),
        }
    }

    pub fn add_sound_input(
        mut self,
        input_id: SoundInputId,
        label: impl Into<String>,
        sound_graph: &SoundGraph,
    ) -> Self {
        let time_snid = sound_graph
            .topology()
            .sound_input(input_id)
            .unwrap()
            .time_argument();
        self.sound_inputs.push((input_id, label.into(), time_snid));
        self
    }

    pub fn add_expression(
        mut self,
        expr_id: SoundExpressionId,
        label: impl Into<String>,
        config: PlotConfig,
    ) -> Self {
        self.expressions.push((expr_id, label.into(), config));
        self
    }

    pub fn add_argument(
        mut self,
        argument_id: SoundExpressionArgumentId,
        label: impl Into<String>,
    ) -> Self {
        self.arguments.push((argument_id, label.into()));
        self
    }

    pub fn show(
        self,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        ui_state: &mut SoundGraphUiState,
        sound_graph: &mut SoundGraph,
    ) {
        self.show_with(
            ui,
            ctx,
            ui_state,
            sound_graph,
            |_ui, _ui_state, _sound_graph| {},
        );
    }

    pub fn show_with<F: FnOnce(&mut egui::Ui, &mut SoundGraphUiState, &mut SoundGraph)>(
        self,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        ui_state: &mut SoundGraphUiState,
        sound_graph: &mut SoundGraph,
        add_contents: F,
    ) {
        for (nsid, name) in &self.arguments {
            ui_state
                .names_mut()
                .record_argument_name(*nsid, name.to_string());
        }
        for (_, _, time_nsid) in &self.sound_inputs {
            ui_state
                .names_mut()
                .record_argument_name(*time_nsid, "time".to_string());
        }

        #[cfg(debug_assertions)]
        {
            let proc_data = sound_graph
                .topology()
                .sound_processor(self.processor_id)
                .unwrap();
            let missing_name =
                |id: SoundExpressionArgumentId| ui_state.names().argument(id).is_none();
            for nsid in proc_data.expression_arguments() {
                if missing_name(*nsid) {
                    println!(
                        "Warning: argument {} on processor {} is missing a name",
                        nsid.value(),
                        proc_data.friendly_name()
                    );
                }
            }
            for siid in proc_data.sound_inputs() {
                for nsid in sound_graph
                    .topology()
                    .sound_input(*siid)
                    .unwrap()
                    .expression_arguments()
                {
                    if missing_name(*nsid) {
                        println!(
                            "Warning: argument {} on sound input {} on processor {} is missing a name",
                            nsid.value(),
                            siid.value(),
                            proc_data.friendly_name()
                        );
                    }
                }
                if self
                    .sound_inputs
                    .iter()
                    .find(|(id, _, _)| *id == *siid)
                    .is_none()
                {
                    println!(
                        "Warning: sound input {} on proceessor {} is not listed in the ui",
                        siid.value(),
                        proc_data.friendly_name()
                    )
                }
            }
        }

        let r = ui.push_id(self.processor_id, |ui| {
            self.show_with_impl(ui, ctx, ui_state, sound_graph, add_contents);
        });

        ui_state.positions_mut().record_processor(
            self.processor_id,
            r.response.rect,
            ctx.group_origin(),
        );
    }

    fn show_with_impl<F: FnOnce(&mut egui::Ui, &mut SoundGraphUiState, &mut SoundGraph)>(
        &self,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        ui_state: &mut SoundGraphUiState,
        sound_graph: &mut SoundGraph,
        add_contents: F,
    ) {
        // Clip to the entire screen, not just outside the area
        // TODO: is this still needed? IIRC this was just to prevent
        // tooltips from getting cut off
        ui.set_clip_rect(ui.ctx().input(|i| i.screen_rect()));

        let darkish_stroke = egui::Stroke::new(2.0, egui::Color32::from_black_alpha(128));

        let frame = egui::Frame::default()
            .fill(self.color)
            .inner_margin(egui::vec2(0.0, 5.0))
            .stroke(darkish_stroke);

        let props = ProcessorUiProps {
            origin: ui.cursor().left_top(),
        };

        let left_of_body = props.origin.x;

        let desired_width = ctx.width();

        for (siid, label, _time_nsid) in &self.sound_inputs {
            ui_state
                .names_mut()
                .record_sound_input_name(*siid, label.to_string());
        }

        ui.set_width(desired_width);

        Self::show_inner_processor_contents(ui, left_of_body, desired_width, frame, |ui| {
            ui.vertical(|ui| {
                // Make sure to use up the intended width consistently
                ui.set_width(desired_width);

                // Show all expressions in order
                for (input_id, input_label, config) in &self.expressions {
                    self.show_expression(
                        ui,
                        ctx,
                        *input_id,
                        input_label,
                        ui_state,
                        sound_graph,
                        config,
                    );
                }

                // Show the processor name and also type name if it differs
                ui.horizontal(|ui| {
                    ui.spacing();
                    let name = ui_state
                        .names()
                        .sound_processor(self.processor_id)
                        .unwrap()
                        .name()
                        .to_string();

                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&name)
                                .color(egui::Color32::BLACK)
                                .strong(),
                        )
                        .wrap(false),
                    );

                    if !name.to_lowercase().contains(&self.label.to_lowercase()) {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(self.label)
                                    .color(egui::Color32::from_black_alpha(192))
                                    .italics(),
                            )
                            .selectable(false),
                        );
                    }
                });

                // Add any per-processor custom contents
                add_contents(ui, ui_state, sound_graph);

                // Check for interactions with the background of the
                // processor so that it can be dragged
                let bg_response = ui.interact_bg(egui::Sense::click_and_drag());

                // Handle drag & drop
                {
                    if bg_response.drag_started() {
                        ui_state.interactions_mut().start_dragging(
                            DragDropSubject::Processor(self.processor_id),
                            bg_response.rect,
                        );
                    }

                    if bg_response.dragged() {
                        ui_state
                            .interactions_mut()
                            .continue_drag_move_by(bg_response.drag_delta());
                    }

                    if bg_response.drag_stopped() {
                        ui_state.interactions_mut().drop_dragging();
                    }
                }

                // Handle click to focus
                if bg_response.clicked() {
                    ui_state
                        .interactions_mut()
                        .focus_on_processor(self.processor_id);
                }
            });
        });
    }

    fn show_inner_processor_contents<F: FnOnce(&mut egui::Ui)>(
        ui: &mut egui::Ui,
        left_of_body: f32,
        desired_width: f32,
        inner_frame: egui::Frame,
        f: F,
    ) -> egui::Response {
        let body_rect = egui::Rect::from_x_y_ranges(
            left_of_body..=(left_of_body + desired_width),
            ui.cursor().top()..=f32::INFINITY,
        );

        ui.allocate_ui_at_rect(body_rect, |ui| {
            ui.set_width(desired_width);
            inner_frame.show(ui, f).response
        })
        .response
    }

    fn show_expression(
        &self,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        expr_id: SoundExpressionId,
        expr_label: &str,
        ui_state: &mut SoundGraphUiState,
        sound_graph: &mut SoundGraph,
        plot_config: &PlotConfig,
    ) {
        let fill = egui::Color32::from_black_alpha(64);

        let expr_frame = egui::Frame::default()
            .fill(fill)
            .inner_margin(egui::vec2(5.0, 5.0))
            .stroke(egui::Stroke::new(2.0, egui::Color32::from_black_alpha(128)));

        let r = expr_frame.show(ui, |ui| {
            ui_state
                .names_mut()
                .record_expression_name(expr_id, expr_label.to_string());

            ui_state.show_expression_graph_ui(expr_id, sound_graph, ctx, plot_config, ui);
        });

        // Track the expression's position
        ui_state
            .positions_mut()
            .record_expression(expr_id, r.response.rect);
    }
}
