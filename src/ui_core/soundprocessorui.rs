use eframe::egui;

use crate::core::{
    sound::{
        soundinput::{InputOptions, SoundInputId},
        soundnumberinput::SoundNumberInputId,
        soundnumbersource::{SoundNumberSourceId, SoundNumberSourceOwner},
        soundprocessor::SoundProcessorId,
    },
    uniqueid::UniqueId,
};

use super::{
    numbergraphuicontext::OuterNumberGraphUiContext, soundgraphuicontext::SoundGraphUiContext,
    soundgraphuistate::SoundGraphUiState, soundnumberinputui::SoundNumberInputUi,
};

pub struct ProcessorUi {
    processor_id: SoundProcessorId,
    label: &'static str,
    color: egui::Color32,
    number_inputs: Vec<(SoundNumberInputId, String)>,
    number_sources: Vec<(SoundNumberSourceId, String)>,
    sound_inputs: Vec<(SoundInputId, String)>,
}

#[derive(Clone, Copy)]
struct ProcessorUiProps {
    origin: egui::Pos2,
    indentation: f32,
}

impl ProcessorUi {
    pub fn new(id: SoundProcessorId, label: &'static str, color: egui::Color32) -> ProcessorUi {
        ProcessorUi {
            processor_id: id,
            label,
            color,
            number_inputs: Vec::new(),
            number_sources: Vec::new(),
            sound_inputs: Vec::new(),
        }
    }

    pub fn add_sound_input(mut self, input_id: SoundInputId, label: impl Into<String>) -> Self {
        self.sound_inputs.push((input_id, label.into()));
        self
    }

    pub fn add_number_input(
        mut self,
        input_id: SoundNumberInputId,
        label: impl Into<String>,
    ) -> Self {
        self.number_inputs.push((input_id, label.into()));
        self
    }

    pub fn add_number_source(
        mut self,
        source_id: SoundNumberSourceId,
        label: impl Into<String>,
    ) -> Self {
        self.number_sources.push((source_id, label.into()));
        self
    }

    const RAIL_WIDTH: f32 = 15.0;

    pub fn show(
        self,
        ui: &mut egui::Ui,
        ctx: &mut SoundGraphUiContext,
        ui_state: &mut SoundGraphUiState,
    ) {
        self.show_with(ui, ctx, ui_state, |_ui, _ui_state| {});
    }

    pub fn show_with<F: FnOnce(&mut egui::Ui, &mut SoundGraphUiState)>(
        self,
        ui: &mut egui::Ui,
        ctx: &mut SoundGraphUiContext,
        ui_state: &mut SoundGraphUiState,
        add_contents: F,
    ) {
        for (nsid, name) in &self.number_sources {
            ui_state.names_mut().record_number_source_name(*nsid, name);
        }

        #[cfg(debug_assertions)]
        {
            let proc_data = ctx.topology().sound_processor(self.processor_id).unwrap();
            let missing_name = |id: SoundNumberSourceId| {
                self.number_sources.iter().find(|(i, _)| *i == id).is_none()
            };
            let processor_type_name = proc_data.instance_arc().as_graph_object().get_type().name();
            for nsid in proc_data.number_sources() {
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
                for nsid in ctx.topology().sound_input(*siid).unwrap().number_sources() {
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
            }
        }

        let response = if ctx.is_top_level() {
            // If the object is top-level, draw it in a new egui::Area,
            // which can be independently clicked and dragged and moved
            // in front of other objects

            let s = format!("SoundProcessorUi {:?}", self.processor_id);
            let id = egui::Id::new(s);

            let mut area = egui::Area::new(id)
                .movable(false) // disable dragging the area directly, since that gets handled below
                .constrain(false)
                .drag_bounds(egui::Rect::EVERYTHING);

            if let Some(state) = ui_state
                .object_positions()
                .get_object_location(self.processor_id.into())
            {
                let pos = state.rect().left_top();
                area = area.current_pos(pos);
            }

            let r = area.show(ui.ctx(), |ui| {
                let r = self.show_with_impl(ui, ctx, ui_state, add_contents);

                self.draw_wires(ui, ctx, ui_state);

                r
            });

            let response = r
                .response
                .interact(egui::Sense::click_and_drag())
                .union(r.inner);

            response
        } else {
            // Otherwise, if the object isn't top-level, nest it within the
            // current egui::Ui

            let response = self.show_with_impl(ui, ctx, ui_state, add_contents);

            if ui_state.dragging_processor_data().map(|x| x.processor_id) == Some(self.processor_id)
            {
                // Make the processor appear faded if it's being dragged. A representation
                // of the processor that follows the cursor will be drawn separately.
                ui.painter().rect_filled(
                    response.rect,
                    egui::Rounding::none(),
                    egui::Color32::from_black_alpha(64),
                );
            }

            response
        };

        if response.drag_started() {
            if !ui_state.is_object_selected(self.processor_id.into()) {
                // Stop selecting, allowing the processor to be dragged onto sound inputs
                ui_state.stop_selecting();
            }
        }

        if response.dragged() {
            let from_input = if ctx.is_top_level() {
                None
            } else {
                Some(ctx.parent_sound_input().unwrap())
            };

            let from_rect = response.rect;

            ui_state.drag_processor(
                self.processor_id,
                response.drag_delta(),
                response.interact_pointer_pos().unwrap(),
                from_input,
                from_rect,
            );
        }

        if response.clicked() {
            if !ui_state.is_object_selected(self.processor_id.into()) {
                ui_state.stop_selecting();
                ui_state.select_object(self.processor_id.into());
            }
        }

        if response.drag_released() {
            ui_state.drop_dragging_processor();
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

    fn show_with_impl<F: FnOnce(&mut egui::Ui, &mut SoundGraphUiState)>(
        &self,
        ui: &mut egui::Ui,
        ctx: &mut SoundGraphUiContext,
        ui_state: &mut SoundGraphUiState,
        add_contents: F,
    ) -> egui::Response {
        // Clip to the entire screen, not just outside the area
        ui.set_clip_rect(ui.ctx().input(|i| i.screen_rect()));

        let fill = self.color;

        let (outer_frame, inner_frame) = Self::outer_and_inner_processor_frames(fill);

        let props = ProcessorUiProps {
            origin: ui.cursor().left_top(),
            indentation: (ctx.nesting_depth() + 1) as f32 * Self::RAIL_WIDTH,
        };

        let left_of_body = props.origin.x + props.indentation;

        let desired_width = ctx.width();

        for (siid, label) in &self.sound_inputs {
            ui_state.names_mut().record_sound_input_name(*siid, label);
        }

        let r = outer_frame.show(ui, |ui| {
            ui.set_width(desired_width);
            if !self.sound_inputs.is_empty() {
                for (input_id, _label) in &self.sound_inputs {
                    self.show_sound_input(ui, ctx, *input_id, ui_state, props);
                }
            }

            let response = Self::show_inner_processor_contents(
                ui,
                left_of_body,
                desired_width,
                inner_frame,
                |ui| {
                    ui.vertical(|ui| {
                        for (input_id, input_label) in &self.number_inputs {
                            self.show_number_input(ui, ctx, *input_id, input_label, ui_state);
                        }
                        ui.set_width(desired_width);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(self.label)
                                    .color(egui::Color32::BLACK)
                                    .strong(),
                            )
                            .wrap(false),
                        );
                        add_contents(ui, ui_state)
                    });
                },
            );

            let top_rail_rect = egui::Rect::from_x_y_ranges(
                props.origin.x..=(props.origin.x + Self::RAIL_WIDTH - 2.0),
                props.origin.y..=response.rect.bottom(),
            );

            let rounding = egui::Rounding::same(3.0);

            ui.painter().rect_filled(top_rail_rect, rounding, fill);
            ui.painter().rect_stroke(
                top_rail_rect,
                rounding,
                egui::Stroke::new(2.0, egui::Color32::from_black_alpha(128)),
            );

            ui_state
                .object_positions_mut()
                .track_processor_rail_location(self.processor_id, top_rail_rect);

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

    fn show_sound_input(
        &self,
        ui: &mut egui::Ui,
        ctx: &mut SoundGraphUiContext,
        input_id: SoundInputId,
        ui_state: &mut SoundGraphUiState,
        mut props: ProcessorUiProps,
    ) {
        let processor_candidacy = ui_state
            .dragging_processor_data()
            .and_then(|d| d.candidate_inputs.get(&input_id));

        let input_data = ctx.topology().sound_input(input_id).unwrap();

        let opts = input_data.options();

        let nonsync_shim_width = Self::RAIL_WIDTH * 0.5;

        let original_origin = props.origin;
        let mut desired_width = ctx.width();

        if let InputOptions::NonSynchronous = opts {
            props.origin.x += nonsync_shim_width;
            desired_width -= nonsync_shim_width;
        }

        let left_of_body = props.origin.x + props.indentation;

        let desired_width = desired_width;

        let top_of_input = ui.cursor().top();

        let input_frame = if processor_candidacy.is_some() {
            egui::Frame::default()
                .fill(egui::Color32::from_white_alpha(64))
                .inner_margin(egui::vec2(0.0, 5.0))
                .stroke(egui::Stroke::new(2.0, egui::Color32::from_white_alpha(128)))
        } else {
            egui::Frame::default()
                .fill(egui::Color32::from_black_alpha(64))
                .inner_margin(egui::vec2(0.0, 5.0))
                .stroke(egui::Stroke::new(2.0, egui::Color32::from_black_alpha(128)))
        };

        let target = input_data.target();
        let r = match target {
            Some(spid) => {
                // move the inner UI one rail's width to the right to account for
                // the lesser nesting level and to let the nested object ui find
                // the correct horizontal extent again
                let inner_objectui_rect = egui::Rect::from_x_y_ranges(
                    (props.origin.x + Self::RAIL_WIDTH)..=f32::INFINITY,
                    ui.cursor().top()..=f32::INFINITY,
                );

                ui.allocate_ui_at_rect(inner_objectui_rect, |ui| {
                    if ui_state.temporal_layout().is_top_level(spid.into()) {
                        let color = ctx.object_states().get_object_color(spid.into());
                        let (outer_frame, inner_frame) =
                            Self::outer_and_inner_processor_frames(color);
                        let response = outer_frame
                            .show(ui, |ui| {
                                Self::show_inner_processor_contents(
                                    ui,
                                    inner_objectui_rect.left() + Self::RAIL_WIDTH,
                                    desired_width,
                                    inner_frame,
                                    |ui| {
                                        ui.horizontal(|ui| {
                                            ui.set_width(desired_width);
                                            ui.add_space(10.0);
                                            let rect = ui.allocate_space(egui::Vec2::splat(20.0)).1;
                                            let origin = rect.center();
                                            let processor_position = ui_state
                                                .object_positions()
                                                .get_object_location(spid.into())
                                                .unwrap()
                                                .rect()
                                                .center();
                                            let vec_to_processor = processor_position - origin;
                                            let vec_to_processor = vec_to_processor
                                                * (10.0 / vec_to_processor.length());
                                            ui.painter().arrow(
                                                origin - vec_to_processor,
                                                2.0 * vec_to_processor,
                                                egui::Stroke::new(2.0, egui::Color32::BLACK),
                                            );
                                        });
                                    },
                                )
                            })
                            .inner;

                        if response.dragged() {
                            let from_input = Some(input_id);
                            let from_rect = response.rect;
                            ui_state.drag_processor(
                                spid,
                                response.drag_delta(),
                                response.interact_pointer_pos().unwrap(),
                                from_input,
                                from_rect,
                            );
                        }

                        if response.drag_released() {
                            ui_state.drop_dragging_processor();
                        }
                    } else {
                        // draw the processor right above
                        let target_processor = ctx.topology().sound_processor(spid).unwrap();
                        let target_graph_object = target_processor.instance_arc().as_graph_object();

                        ctx.show_nested_ui(
                            input_id,
                            desired_width,
                            &target_graph_object,
                            ui_state,
                            ui,
                        );
                    }
                })
            }
            None => {
                // move the inner UI exactly to the desired horizontal extent,
                // past all rails, where it actually needs to get drawn
                let input_rect = egui::Rect::from_x_y_ranges(
                    left_of_body..=(left_of_body + desired_width),
                    ui.cursor().top()..=f32::INFINITY,
                );

                ui.allocate_ui_at_rect(input_rect, |ui| {
                    // TODO: draw an empty field onto which things can be dragged
                    input_frame.show(ui, |ui| {
                        ui.set_width(desired_width);
                        let label_str = format!("Sound Input {} (empty)", input_id.value(),);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(label_str)
                                    .color(egui::Color32::BLACK)
                                    .strong(),
                            )
                            .wrap(false),
                        );
                    });
                })
            }
        };

        ui_state
            .object_positions_mut()
            .track_sound_input_location(input_id, r.response.rect);

        let bottom_of_input = ui.cursor().top();

        if let InputOptions::NonSynchronous = opts {
            let left_of_shim = original_origin.x + Self::RAIL_WIDTH;
            let nonsync_shim_rect = egui::Rect::from_x_y_ranges(
                left_of_shim..=(left_of_shim + nonsync_shim_width),
                top_of_input..=bottom_of_input,
            );
            ui.painter().rect_filled(
                nonsync_shim_rect,
                egui::Rounding::none(),
                egui::Color32::from_black_alpha(64),
            );
        }

        if ui_state.is_item_focused(input_id.into()) {
            ui.painter().rect_stroke(
                r.response.rect,
                egui::Rounding::same(3.0),
                egui::Stroke::new(2.0, egui::Color32::YELLOW),
            );
        }
    }

    fn show_number_input(
        &self,
        ui: &mut egui::Ui,
        ctx: &mut SoundGraphUiContext,
        input_id: SoundNumberInputId,
        input_label: &str,
        ui_state: &mut SoundGraphUiState,
    ) {
        let fill = egui::Color32::from_black_alpha(64);

        let input_frame = egui::Frame::default()
            .fill(fill)
            .inner_margin(egui::vec2(0.0, 5.0))
            .stroke(egui::Stroke::new(2.0, egui::Color32::from_black_alpha(128)));

        let res = input_frame.show(ui, |ui| {
            ui.set_width(ctx.width());

            let input_ui = SoundNumberInputUi::new(input_id);

            let (number_ui_state, presentation, focus, temporal_layout, names) =
                ui_state.number_graph_ui_parts(input_id);

            let graph_input_references = ctx.with_number_graph_ui_context(
                input_id,
                temporal_layout,
                names,
                |number_ctx, sni_ctx| {
                    let mut outer_ctx: OuterNumberGraphUiContext = sni_ctx.into();
                    input_ui.show(
                        ui,
                        input_label,
                        number_ui_state,
                        number_ctx,
                        presentation,
                        focus,
                        &mut outer_ctx,
                    )
                },
            );

            graph_input_references
        });

        let graph_input_references = res.inner;

        ui_state
            .object_positions_mut()
            .track_sound_number_input_location(input_id, res.response.rect);

        ctx.number_graph_input_references()
            .borrow_mut()
            .push((input_id, graph_input_references));

        if ui_state.is_item_focused(input_id.into()) {
            ui.painter().rect_stroke(
                res.response.rect,
                egui::Rounding::same(3.0),
                egui::Stroke::new(2.0, egui::Color32::YELLOW),
            );
        }
    }

    fn draw_wires(
        &self,
        ui: &mut egui::Ui,
        ctx: &mut SoundGraphUiContext,
        ui_state: &mut SoundGraphUiState,
    ) {
        let references = ctx.number_graph_input_references().borrow();

        for (number_input_id, graph_input_references) in references.iter() {
            for graph_input in graph_input_references.iter() {
                let target_number_source = ctx
                    .topology()
                    .number_input(*number_input_id)
                    .unwrap()
                    .target_mapping()
                    .graph_input_target(graph_input.input_id())
                    .unwrap();
                let target_processor = match ctx
                    .topology()
                    .number_source(target_number_source)
                    .unwrap()
                    .owner()
                {
                    SoundNumberSourceOwner::SoundProcessor(spid) => spid,
                    SoundNumberSourceOwner::SoundInput(siid) => {
                        ctx.topology().sound_input(siid).unwrap().owner()
                    }
                };
                let target_rail_location = ui_state
                    .object_positions()
                    .get_processor_rail_location(target_processor)
                    .unwrap();

                let y = graph_input.location().y;

                let x_begin = target_rail_location.rect().left() + 1.0;
                let x_end = graph_input.location().x;

                let wire_rect = egui::Rect::from_x_y_ranges(x_begin..=x_end, y..=(y + 8.0));
                let wire_color = ctx
                    .object_states()
                    .get_apparent_object_color(target_processor.into(), ui_state);

                ui.painter().rect(
                    wire_rect,
                    egui::Rounding::same(4.0),
                    wire_color,
                    egui::Stroke::new(1.0, egui::Color32::from_black_alpha(128)),
                );
            }
        }
    }
}
