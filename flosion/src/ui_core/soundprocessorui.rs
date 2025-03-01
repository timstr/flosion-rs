use eframe::egui;

use crate::{
    core::{
        samplefrequency::SAMPLE_FREQUENCY,
        sound::{
            argument::{AnyProcessorArgument, ProcessorArgumentId, ProcessorArgumentLocation},
            expression::{ProcessorExpression, ProcessorExpressionId, ProcessorExpressionLocation},
            soundinput::{AnyProcessorInput, ProcessorInputId, SoundInputLocation},
            soundprocessor::{AnySoundProcessor, ProcessorComponentVisitor, SoundProcessorId},
        },
    },
    ui_core::soundgraphuinames::SoundGraphUiNames,
};

use super::{
    expressionplot::PlotConfig, interactions::draganddrop::DragDropSubject,
    soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
};

pub struct ProcessorUi {
    label: &'static str,
    expressions: Vec<(ProcessorExpressionId, Vec<String>, PlotConfig)>,
    arguments: Vec<(ProcessorArgumentId, String)>,
    sound_inputs: Vec<(ProcessorInputId, String)>,
}

#[derive(Clone, Copy)]
struct ProcessorUiProps {
    // Huh?
}

impl ProcessorUi {
    pub fn new(label: &'static str) -> ProcessorUi {
        ProcessorUi {
            label,
            expressions: Vec::new(),
            arguments: Vec::new(),
            sound_inputs: Vec::new(),
        }
    }

    pub fn add_sound_input<I: AnyProcessorInput>(
        mut self,
        input: &I,
        label: impl Into<String>,
    ) -> Self {
        self.sound_inputs.push((input.id(), label.into()));
        self
    }

    pub fn add_expression(
        mut self,
        expr: &ProcessorExpression,
        labels: &[&str],
        config: PlotConfig,
    ) -> Self {
        self.expressions.push((
            expr.id(),
            labels.iter().map(|l| l.to_string()).collect(),
            config,
        ));
        self
    }

    pub fn add_argument<A: AnyProcessorArgument>(
        mut self,
        argument: &A,
        label: impl Into<String>,
    ) -> Self {
        self.arguments.push((argument.id(), label.into()));
        self
    }

    pub fn show<T: AnySoundProcessor>(
        self,
        processor: &mut T,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        ui_state: &mut SoundGraphUiState,
    ) {
        self.show_with(
            processor,
            ui,
            ctx,
            ui_state,
            |_processor, _ui, _ui_state| {},
        );
    }

    pub fn show_with<
        T: AnySoundProcessor,
        F: FnOnce(&mut T, &mut egui::Ui, &mut SoundGraphUiState),
    >(
        mut self,
        processor: &mut T,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        ui_state: &mut SoundGraphUiState,
        add_contents: F,
    ) {
        for (arg_id, name) in &self.arguments {
            ui_state.names_mut().record_argument_name(
                ProcessorArgumentLocation::new(processor.id(), *arg_id),
                name.to_string(),
            );
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
                fn input(&mut self, input: &dyn AnyProcessorInput) {
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
                    for result in expression.graph().results() {
                        if self
                            .names
                            .expression_result(location, result.id())
                            .is_none()
                        {
                            println!(
                                "Warning: result expression {} on processor {} is missing a result name",
                                location.expression().value(),
                                self.friendly_processor_name
                            );
                        }
                    }
                }

                fn argument(&mut self, argument: &dyn AnyProcessorArgument) {
                    let location = ProcessorArgumentLocation::new(self.processor_id, argument.id());
                    if self.names.argument(location.into()).is_none() {
                        println!(
                            "Warning: argument {} on processor {} is missing a name",
                            location.argument().value(),
                            self.friendly_processor_name
                        );
                    }
                }
            }

            let mut visitor = MissingNameVisitor {
                names: ui_state.names(),
                processor_id: processor.id(),
                friendly_processor_name: processor.as_graph_object().friendly_name(),
            };

            processor.visit(&mut visitor);
        }

        ui.push_id(processor.id(), |ui| {
            self.show_with_impl(processor, ui, ctx, ui_state, add_contents);
        });
    }

    fn show_with_impl<
        T: AnySoundProcessor,
        F: FnOnce(&mut T, &mut egui::Ui, &mut SoundGraphUiState),
    >(
        &mut self,
        processor: &mut T,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        ui_state: &mut SoundGraphUiState,
        add_contents: F,
    ) {
        // Clip to the entire screen, not just outside the area
        // TODO: is this still needed? IIRC this was just to prevent
        // tooltips from getting cut off
        ui.set_clip_rect(ui.ctx().input(|i| i.screen_rect()));

        let darkish_stroke = egui::Stroke::new(2.0, egui::Color32::from_black_alpha(128));

        let color = ui_state
            .object_states()
            .get_object_color(processor.id().into());

        let frame = egui::Frame::default()
            .fill(color)
            .inner_margin(egui::vec2(0.0, 5.0))
            .stroke(darkish_stroke);

        let desired_width = ctx.width();

        for (siid, label) in &self.sound_inputs {
            ui_state.names_mut().record_sound_input_name(
                SoundInputLocation::new(processor.id(), *siid),
                label.to_string(),
            );
        }

        ui.set_width(desired_width);

        let frame_response = frame.show(ui, |ui| {
            ui.vertical(|ui| {
                // Make sure to use up the intended width consistently
                ui.set_width(desired_width);

                // Show all expressions in order
                for (expr_id, result_labels, config) in &mut self.expressions {
                    Self::show_expression(
                        processor,
                        ui,
                        ctx,
                        *expr_id,
                        result_labels,
                        ui_state,
                        config,
                    );
                }

                // Show the processor name and also type name if it differs
                ui.horizontal(|ui| {
                    ui.spacing();
                    let name = ui_state.names().sound_processor(processor.id()).unwrap();
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(name)
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
                add_contents(processor, ui, ui_state);

                // Check for interactions with the background of the
                // processor so that it can be dragged
                let bg_response = ui.interact_bg(egui::Sense::click_and_drag());

                // Handle drag & drop
                {
                    if bg_response.drag_started() {
                        ui_state.interactions_mut().start_dragging(
                            DragDropSubject::Processor(processor.id()),
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
                        .focus_on_processor(processor.id());
                    ctx.request_snapshot();
                }
            });
        });

        let frame_rect = frame_response.response.rect;

        if let Some(report) = ctx.compiled_processor_report(processor.id()) {
            for time_samples in report.times_samples() {
                let time = *time_samples as f32 / SAMPLE_FREQUENCY as f32;
                let x = frame_rect.left() + (time / ctx.time_axis().seconds_per_x_pixel);

                if x > frame_rect.right() {
                    continue;
                }

                let painter = ui.painter();
                painter.line_segment(
                    [
                        egui::pos2(x, frame_rect.top()),
                        egui::pos2(x, frame_rect.bottom()),
                    ],
                    egui::Stroke::new(2.0, egui::Color32::from_white_alpha(64)),
                );
            }

            ui.ctx().request_repaint();
        }
    }

    fn show_expression(
        processor: &mut dyn AnySoundProcessor,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        expr_id: ProcessorExpressionId,
        result_labels: &[String],
        ui_state: &mut SoundGraphUiState,
        plot_config: &PlotConfig,
    ) {
        let fill = egui::Color32::from_black_alpha(64);

        let expr_frame = egui::Frame::default()
            .fill(fill)
            .inner_margin(egui::vec2(5.0, 5.0))
            .stroke(egui::Stroke::new(2.0, egui::Color32::from_black_alpha(128)));

        let location = ProcessorExpressionLocation::new(processor.id(), expr_id);

        let r = expr_frame.show(ui, |ui| {
            let processor_id = processor.id();
            processor.with_expression_mut(expr_id, |expr| {
                assert_eq!(
                    result_labels.len(),
                    expr.graph().results().len(),
                    "Passed {} result name(s) for an expression graph which has {} result(s)",
                    result_labels.len(),
                    expr.graph().results().len()
                );
                for (result, name) in expr.graph().results().iter().zip(result_labels) {
                    ui_state.names_mut().record_expression_result_name(
                        location,
                        result.id(),
                        name.to_string(),
                    );
                }
                ui_state.show_expression_graph_ui(
                    processor_id,
                    expr,
                    ctx,
                    plot_config,
                    ui,
                    ctx.snapshot_flag(),
                );
            });
        });

        // Track the expression's position
        ui_state
            .positions_mut()
            .record_expression(location, r.response.rect);
    }
}
