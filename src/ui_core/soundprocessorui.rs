use eframe::egui;

use crate::core::{
    sound::{
        expression::SoundExpressionId,
        expressionargument::SoundExpressionArgumentId,
        soundgraph::SoundGraph,
        soundinput::SoundInputId,
        soundprocessor::{ProcessorHandle, SoundProcessorId},
    },
    uniqueid::UniqueId,
};

use super::{
    expressionplot::PlotConfig, soundgraphuicontext::SoundGraphUiContext,
    soundgraphuistate::SoundGraphUiState,
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
        input_id: SoundExpressionId,
        label: impl Into<String>,
        config: PlotConfig,
    ) -> Self {
        self.expressions.push((input_id, label.into(), config));
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
            let processor_type_name = proc_data.instance_arc().as_graph_object().get_type().name();
            for nsid in proc_data.expression_arguments() {
                if missing_name(*nsid) {
                    println!(
                        "Warning: argument {} on processor {} ({}) is missing a name",
                        nsid.value(),
                        self.processor_id.value(),
                        processor_type_name
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
                            "Warning: argument {} on sound input {} on processor {} ({}) is missing a name",
                            nsid.value(),
                            siid.value(),
                            self.processor_id.value(),
                            processor_type_name
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
                        "Warning: sound input {} on proceessor {} ({}) is not listed in the ui",
                        siid.value(),
                        self.processor_id.value(),
                        processor_type_name
                    )
                }
            }
        }

        self.show_with_impl(ui, ctx, ui_state, sound_graph, add_contents);
    }

    fn show_with_impl<F: FnOnce(&mut egui::Ui, &mut SoundGraphUiState, &mut SoundGraph)>(
        &self,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        ui_state: &mut SoundGraphUiState,
        sound_graph: &mut SoundGraph,
        add_contents: F,
    ) -> egui::Response {
        // Clip to the entire screen, not just outside the area
        ui.set_clip_rect(ui.ctx().input(|i| i.screen_rect()));

        let darkish_stroke = egui::Stroke::new(2.0, egui::Color32::from_black_alpha(128));

        let frame = egui::Frame::default()
            .fill(self.color)
            .inner_margin(egui::vec2(0.0, 5.0))
            .rounding(5.0)
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

        let response =
            Self::show_inner_processor_contents(ui, left_of_body, desired_width, frame, |ui| {
                ui.vertical(|ui| {
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
                    ui.set_width(desired_width);

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
                            ui.add(egui::Label::new(
                                egui::RichText::new(self.label)
                                    .color(egui::Color32::from_black_alpha(192))
                                    .italics(),
                            ));
                        }
                    });
                    add_contents(ui, ui_state, sound_graph)
                });
            });

        response
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

        let r = ui.allocate_ui_at_rect(body_rect, |ui| {
            ui.set_width(desired_width);
            inner_frame.show(ui, f).response
        });

        let bottom_of_body = ui.cursor().top();

        let body_rect = body_rect.intersect(egui::Rect::everything_above(bottom_of_body));

        // check for click/drag interactions with the background of the processor body
        r.response
            .with_new_rect(body_rect)
            .interact(egui::Sense::click_and_drag())
    }

    fn show_expression(
        &self,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        input_id: SoundExpressionId,
        input_label: &str,
        ui_state: &mut SoundGraphUiState,
        sound_graph: &mut SoundGraph,
        plot_config: &PlotConfig,
    ) {
        let fill = egui::Color32::from_black_alpha(64);

        let expr_frame = egui::Frame::default()
            .fill(fill)
            .inner_margin(egui::vec2(5.0, 5.0))
            .stroke(egui::Stroke::new(2.0, egui::Color32::from_black_alpha(128)));

        expr_frame.show(ui, |ui| {
            ui.set_width(ctx.width());

            ui_state
                .names_mut()
                .record_expression_name(input_id, input_label.to_string());

            ui_state.show_expression_graph_ui(input_id, sound_graph, ctx, plot_config, ui);
        });
    }
}
