use eframe::egui;

use crate::{
    core::{
        sound::{
            expression::SoundExpressionId,
            expressionargument::SoundExpressionArgumentId,
            soundgraph::SoundGraph,
            soundinput::SoundInputId,
            soundprocessor::{ProcessorHandle, SoundProcessorId},
        },
        uniqueid::UniqueId,
    },
    ui_core::soundgraphuinames::SoundGraphUiNames,
};

use super::{
    keyboardfocus::KeyboardFocusState, numbergraphuicontext::OuterNumberGraphUiContext,
    numberinputplot::PlotConfig, soundgraphuicontext::SoundGraphUiContext,
    soundgraphuistate::SoundGraphUiState, soundnumberinputui::SoundNumberInputUi,
};

pub struct ProcessorUi {
    processor_id: SoundProcessorId,
    label: &'static str,
    color: egui::Color32,
    number_inputs: Vec<(SoundExpressionId, String, PlotConfig)>,
    number_sources: Vec<(SoundExpressionArgumentId, String)>,
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
        let mut number_sources = Vec::new();
        number_sources.push((handle.time_argument(), "time".to_string()));
        ProcessorUi {
            processor_id: handle.id(),
            label,
            color,
            number_inputs: Vec::new(),
            number_sources,
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

    pub fn add_number_input(
        mut self,
        input_id: SoundExpressionId,
        label: impl Into<String>,
        config: PlotConfig,
    ) -> Self {
        self.number_inputs.push((input_id, label.into(), config));
        self
    }

    pub fn add_number_source(
        mut self,
        source_id: SoundExpressionArgumentId,
        label: impl Into<String>,
    ) -> Self {
        self.number_sources.push((source_id, label.into()));
        self
    }

    pub fn show(
        self,
        ui: &mut egui::Ui,
        ctx: &mut SoundGraphUiContext,
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
        ctx: &mut SoundGraphUiContext,
        ui_state: &mut SoundGraphUiState,
        sound_graph: &mut SoundGraph,
        add_contents: F,
    ) {
        for (nsid, name) in &self.number_sources {
            ui_state
                .names_mut()
                .record_number_source_name(*nsid, name.to_string());
        }
        for (_, _, time_nsid) in &self.sound_inputs {
            ui_state
                .names_mut()
                .record_number_source_name(*time_nsid, "time".to_string());
        }

        #[cfg(debug_assertions)]
        {
            let proc_data = sound_graph
                .topology()
                .sound_processor(self.processor_id)
                .unwrap();
            let missing_name =
                |id: SoundExpressionArgumentId| ui_state.names().number_source(id).is_none();
            let processor_type_name = proc_data.instance_arc().as_graph_object().get_type().name();
            for nsid in proc_data.expression_arguments() {
                if missing_name(*nsid) {
                    println!(
                        "Warning: number source {} on processor {} ({}) is missing a name",
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
                            "Warning: number source {} on sound input {} on processor {} ({}) is missing a name",
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

        let response = self.show_with_impl(ui, ctx, ui_state, sound_graph, add_contents);

        if todo!("is this processor being dragged?") {
            // Make the processor appear faded if it's being dragged. A representation
            // of the processor that follows the cursor will be drawn separately.
            ui.painter().rect_filled(
                response.rect,
                egui::Rounding::ZERO,
                egui::Color32::from_black_alpha(64),
            );
        }

        if response.drag_started() {
            if !ui_state.is_object_selected(self.processor_id.into()) {
                // Stop selecting, allowing the processor to be dragged onto sound inputs
                ui_state.stop_selecting();
            }
        }

        if response.dragged() {
            let from_input = ctx.parent_sound_input();

            let from_rect = response.rect;

            todo!("Start dragging this processor");
        }

        if response.clicked() {
            if !ui_state.is_object_selected(self.processor_id.into()) {
                ui_state.stop_selecting();
                ui_state.select_object(self.processor_id.into());
            }
        }

        if response.drag_stopped() {
            todo!("Stop dragging and drop this processor");
        }
    }

    fn outer_and_inner_processor_frames(color: egui::Color32) -> (egui::Frame, egui::Frame) {
        let darkish_stroke = egui::Stroke::new(2.0, egui::Color32::from_black_alpha(128));

        let outer_frame = egui::Frame::default()
            .fill(egui::Color32::from_rgb(
                (color.r() as u16 * 3 / 4) as u8,
                (color.g() as u16 * 3 / 4) as u8,
                (color.b() as u16 * 3 / 4) as u8,
            ))
            .inner_margin(egui::vec2(0.0, 5.0))
            .stroke(darkish_stroke);

        let inner_frame = egui::Frame::default()
            .fill(color)
            .inner_margin(egui::vec2(0.0, 5.0))
            .stroke(darkish_stroke);

        (outer_frame, inner_frame)
    }

    fn show_with_impl<F: FnOnce(&mut egui::Ui, &mut SoundGraphUiState, &mut SoundGraph)>(
        &self,
        ui: &mut egui::Ui,
        ctx: &mut SoundGraphUiContext,
        ui_state: &mut SoundGraphUiState,
        sound_graph: &mut SoundGraph,
        add_contents: F,
    ) -> egui::Response {
        // Clip to the entire screen, not just outside the area
        ui.set_clip_rect(ui.ctx().input(|i| i.screen_rect()));

        let fill = self.color;

        let (outer_frame, inner_frame) = Self::outer_and_inner_processor_frames(fill);

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

        let r = outer_frame.show(ui, |ui| {
            ui.set_width(desired_width);

            let response = Self::show_inner_processor_contents(
                ui,
                left_of_body,
                desired_width,
                inner_frame,
                |ui| {
                    ui.vertical(|ui| {
                        for (input_id, input_label, config) in &self.number_inputs {
                            self.show_number_input(
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
                        let keyboard_focus_on_name = match ui_state.keyboard_focus() {
                            Some(kbd) => {
                                if let KeyboardFocusState::OnSoundProcessorName(spid) = kbd {
                                    *spid == self.processor_id
                                } else {
                                    false
                                }
                            }
                            None => false,
                        };

                        ui.horizontal(|ui| {
                            ui.spacing();
                            let name = ui_state
                                .names()
                                .sound_processor(self.processor_id)
                                .unwrap()
                                .name()
                                .to_string();

                            if keyboard_focus_on_name {
                                let mut name = name.clone();
                                let r = ui.add(egui::TextEdit::singleline(&mut name));
                                r.request_focus();
                                if r.changed() {
                                    ui_state
                                        .names_mut()
                                        .record_sound_processor_name(self.processor_id, name);
                                }
                                ui.painter().rect_stroke(
                                    r.rect,
                                    egui::Rounding::ZERO,
                                    egui::Stroke::new(2.0, egui::Color32::YELLOW),
                                );
                            } else {
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&name)
                                            .color(egui::Color32::BLACK)
                                            .strong(),
                                    )
                                    .wrap(false),
                                );
                            }
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
                },
            );

            response
        });

        if ui_state.is_item_focused(self.processor_id.into())
            || ui_state.is_object_selected(self.processor_id.into())
        {
            ui.painter().rect_stroke(
                r.response.rect,
                egui::Rounding::same(3.0),
                egui::Stroke::new(2.0, egui::Color32::YELLOW),
            );
        }

        ui_state
            .object_positions_mut()
            .track_object_location(self.processor_id.into(), r.response.rect);

        r.response.union(r.inner)
        // r
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

    fn show_number_input(
        &self,
        ui: &mut egui::Ui,
        ctx: &mut SoundGraphUiContext,
        input_id: SoundExpressionId,
        input_label: &str,
        ui_state: &mut SoundGraphUiState,
        sound_graph: &mut SoundGraph,
        config: &PlotConfig,
    ) {
        let fill = egui::Color32::from_black_alpha(64);

        let input_frame = egui::Frame::default()
            .fill(fill)
            .inner_margin(egui::vec2(5.0, 5.0))
            .stroke(egui::Stroke::new(2.0, egui::Color32::from_black_alpha(128)));

        let res = input_frame.show(ui, |ui| {
            ui.set_width(ctx.width());

            let input_ui = SoundNumberInputUi::new(input_id);

            let names: &mut SoundGraphUiNames = todo!("Get mutable ref to names");

            names.record_number_input_name(input_id, input_label.to_string());

            todo!("render number graph ui");
            // ctx.with_number_graph_ui_context(
            //     input_id,
            //     temporal_layout,
            //     names,
            //     sound_graph,
            //     |number_ctx, sni_ctx| {
            //         let mut outer_ctx: OuterNumberGraphUiContext = sni_ctx.into();
            //         input_ui.show(
            //             ui,
            //             number_ui_state,
            //             number_ctx,
            //             presentation,
            //             focus,
            //             &mut outer_ctx,
            //             config,
            //         )
            //     },
            // );
        });

        ui_state
            .object_positions_mut()
            .track_sound_number_input_location(input_id, res.response.rect);

        if ui_state.is_item_focused(input_id.into()) {
            ui.painter().rect_stroke(
                res.response.rect,
                egui::Rounding::same(3.0),
                egui::Stroke::new(2.0, egui::Color32::YELLOW),
            );
        }
    }
}
