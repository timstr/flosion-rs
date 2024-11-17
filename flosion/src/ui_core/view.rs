use eframe::egui;

pub(crate) struct View {
    zoom: f32,
    center: egui::Vec2,
}

impl View {
    pub(crate) fn new() -> View {
        View {
            zoom: 1.0,
            center: egui::vec2(0.0, 0.0),
        }
    }

    pub(crate) fn offset(&self, ui: &egui::Ui) -> egui::Vec2 {
        self.center + 0.5 * ui.ctx().screen_rect().size()
    }

    pub(crate) fn handle_pan_and_zoom(&mut self, ui: &mut egui::Ui) {
        let bg_response = ui.response().interact(egui::Sense::click_and_drag());

        if bg_response.dragged_by(egui::PointerButton::Secondary) {
            self.center += bg_response.drag_delta();
        }

        let (scroll, pointer_pos) = ui.input(|i| (i.smooth_scroll_delta, i.pointer.interact_pos()));

        if scroll.y != 0.0 {
            let prev_zoom = self.zoom;
            self.zoom = (self.zoom * (scroll.y * 0.01).exp()).clamp(0.25, 4.0);
            let zoom_ratio = self.zoom / prev_zoom;

            let screen_rect = ui.ctx().screen_rect();
            let screen_half_size = 0.5 * screen_rect.size();

            let pointer_pos = pointer_pos.unwrap_or(screen_half_size.to_pos2());

            let delta_view_center = (pointer_pos.to_vec2() - screen_half_size) * (1.0 - zoom_ratio);
            self.center += delta_view_center;

            ui.ctx().set_zoom_factor(self.zoom);
        }
    }
}
