use eframe::egui;

use super::soundgraphlayout::ProcessorInterconnect;

#[derive(Clone, Copy)]
pub(crate) struct InterconnectPosition {
    pub(crate) interconnect: ProcessorInterconnect,
    pub(crate) rect: egui::Rect,
}

pub(crate) struct SoundObjectPositions {
    interconnects: Vec<InterconnectPosition>,
}

impl SoundObjectPositions {
    pub(crate) fn new() -> SoundObjectPositions {
        SoundObjectPositions {
            interconnects: Vec::new(),
        }
    }

    pub(crate) fn record_interconnect(
        &mut self,
        interconnect: ProcessorInterconnect,
        rect: egui::Rect,
    ) {
        self.interconnects
            .push(InterconnectPosition { interconnect, rect });
    }

    pub(crate) fn find_closest_interconnect(
        &self,
        query: egui::Rect,
        minimum_intersection: f32,
    ) -> Option<InterconnectPosition> {
        let mut best_intersection = minimum_intersection;
        let mut best_interconnect = None;
        for interconnect in &self.interconnects {
            let intersection = interconnect.rect.intersect(query).area();
            if intersection > best_intersection {
                best_intersection = intersection;
                best_interconnect = Some(*interconnect);
            }
        }
        best_interconnect
    }

    pub(crate) fn clear(&mut self) {
        self.interconnects.clear();
    }
}
