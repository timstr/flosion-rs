use eframe::egui;

use crate::{
    core::sound::{
        expression::{ProcessorExpression, ProcessorExpressionLocation},
        expressionargument::{
            ArgumentLocation, ProcessorArgument, ProcessorArgumentId, ProcessorArgumentLocation,
            SoundInputArgument, SoundInputArgumentId, SoundInputArgumentLocation,
        },
        soundgraph::SoundGraph,
        soundinput::{ProcessorInput, ProcessorInputId, SoundInputLocation},
        soundprocessor::{ProcessorComponentVisitor, SoundProcessorId},
    },
    ui_core::soundgraphuinames::SoundGraphUiNames,
};

use super::{
    expressionplot::PlotConfig, interactions::draganddrop::DragDropSubject,
    soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
};

pub struct ProcessorUi<'a> {
    processor_id: SoundProcessorId,
    label: &'static str,
    expressions: Vec<(&'a mut ProcessorExpression, String, PlotConfig)>,
    arguments: Vec<(ArgumentLocation, String)>,
    sound_inputs: Vec<(SoundInputLocation, String)>,
}

#[derive(Clone, Copy)]
struct ProcessorUiProps {
    origin: egui::Pos2,
}

impl<'a> ProcessorUi<'a> {
    pub fn new(processor_id: SoundProcessorId, label: &'static str) -> ProcessorUi<'a> {
        ProcessorUi {
            processor_id,
            label,
            expressions: Vec::new(),
            arguments: Vec::new(),
            sound_inputs: Vec::new(),
        }
    }

    pub fn add_sound_input(mut self, input_id: ProcessorInputId, label: impl Into<String>) -> Self {
        self.sound_inputs.push((
            SoundInputLocation::new(self.processor_id, input_id),
            label.into(),
        ));
        self
    }

    pub fn add_expression(
        mut self,
        expr: &'a mut ProcessorExpression,
        label: impl Into<String>,
        config: PlotConfig,
    ) -> Self {
        self.expressions.push((expr, label.into(), config));
        self
    }

    pub fn add_processor_argument(
        mut self,
        argument_id: ProcessorArgumentId,
        label: impl Into<String>,
    ) -> Self {
        let location = ProcessorArgumentLocation::new(self.processor_id, argument_id);
        self.arguments.push((location.into(), label.into()));
        self
    }

    pub fn add_input_argument(
        mut self,
        input_id: ProcessorInputId,
        argument_id: SoundInputArgumentId,
        label: impl Into<String>,
    ) -> Self {
        let location = SoundInputArgumentLocation::new(self.processor_id, input_id, argument_id);
        self.arguments.push((location.into(), label.into()));
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
        mut self,
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

        // detect missing names
        #[cfg(debug_assertions)]
        {
            struct MissingNameVisitor<'a> {
                names: &'a SoundGraphUiNames,
                processor_id: SoundProcessorId,
                friendly_processor_name: String,
            }

            impl<'a> ProcessorComponentVisitor for MissingNameVisitor<'a> {
                fn input(&mut self, input: &ProcessorInput) {
                    let location = SoundInputLocation::new(self.processor_id, input.id());
                    if self.names.sound_input(location.into()).is_none() {
                        println!(
                            "Warning: sound input {} on processor {} is missing a name",
                            location.input().value(),
                            self.friendly_processor_name
                        );
                    }
                }

                fn expression(&mut self, expression: &ProcessorExpression) {
                    let location =
                        ProcessorExpressionLocation::new(self.processor_id, expression.id());
                    if self.names.expression(location.into()).is_none() {
                        println!(
                            "Warning: expression {} on processor {} is missing a name",
                            location.expression().value(),
                            self.friendly_processor_name
                        );
                    }
                }

                fn processor_argument(&mut self, argument: &ProcessorArgument) {
                    let location = ProcessorArgumentLocation::new(self.processor_id, argument.id());
                    if self.names.argument(location.into()).is_none() {
                        println!(
                            "Warning: argument {} on processor {} is missing a name",
                            location.argument().value(),
                            self.friendly_processor_name
                        );
                    }
                }

                fn input_argument(
                    &mut self,
                    argument: &SoundInputArgument,
                    input_id: ProcessorInputId,
                ) {
                    let location =
                        SoundInputArgumentLocation::new(self.processor_id, input_id, argument.id());
                    if self.names.argument(location.into()).is_none() {
                        println!(
                            "Warning: argument {} on sound input {} of processor {} is missing a name",
                            location.argument().value(),
                            location.input().value(),
                            self.friendly_processor_name
                        );
                    }
                }
            }

            let proc_data = sound_graph.sound_processor(self.processor_id).unwrap();

            let mut visitor = MissingNameVisitor {
                names: ui_state.names(),
                processor_id: proc_data.id(),
                friendly_processor_name: proc_data.friendly_name(),
            };

            proc_data.instance().visit(&mut visitor);
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
        &mut self,
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

        let color = ui_state
            .object_states()
            .get_object_color(self.processor_id.into());

        let frame = egui::Frame::default()
            .fill(color)
            .inner_margin(egui::vec2(0.0, 5.0))
            .stroke(darkish_stroke);

        let props = ProcessorUiProps {
            origin: ui.cursor().left_top(),
        };

        let left_of_body = props.origin.x;

        let desired_width = ctx.width();

        for (siid, label) in &self.sound_inputs {
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
                for (expr, input_label, config) in &mut self.expressions {
                    Self::show_expression(
                        self.processor_id,
                        ui,
                        ctx,
                        *expr,
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
                        .wrap_mode(egui::TextWrapMode::Extend),
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
        processor_id: SoundProcessorId,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        expr: &mut ProcessorExpression,
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

        let location = ProcessorExpressionLocation::new(processor_id, expr.id());

        let r = expr_frame.show(ui, |ui| {
            ui_state
                .names_mut()
                .record_expression_name(location, expr_label.to_string());

            ui_state.show_expression_graph_ui(
                processor_id,
                expr,
                sound_graph,
                ctx,
                plot_config,
                ui,
            );
        });

        // Track the expression's position
        ui_state
            .positions_mut()
            .record_expression(location, r.response.rect);
    }
}
