use eframe::egui;

use crate::core::sound::soundprocessor::SoundProcessorId;

use super::soundgraphlayout::ProcessorInterconnect;

#[derive(Clone, Copy)]
pub(crate) struct InterconnectPosition {
    pub(crate) interconnect: ProcessorInterconnect,
    pub(crate) rect: egui::Rect,
}

pub(crate) struct ProcessorPosition {
    /// The id of the processor
    pub(crate) processor: SoundProcessorId,

    // The on-screen area occupied by the processor's UI
    pub(crate) rect: egui::Rect,

    // The top-left corner of the stacked group currently containing the processor
    pub(crate) group_origin: egui::Pos2,
}

pub(crate) struct SoundObjectPositions {
    interconnects: Vec<InterconnectPosition>,
    processors: Vec<ProcessorPosition>,
}

impl SoundObjectPositions {
    pub(crate) fn new() -> SoundObjectPositions {
        SoundObjectPositions {
            interconnects: Vec::new(),
            processors: Vec::new(),
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

    pub(crate) fn processors(&self) -> &[ProcessorPosition] {
        &self.processors
    }

    pub(crate) fn record_processor(
        &mut self,
        processor: SoundProcessorId,
        rect: egui::Rect,
        group_origin: egui::Pos2,
    ) {
        self.processors.push(ProcessorPosition {
            processor,
            rect,
            group_origin,
        });
    }

    pub(crate) fn find_processor(&self, processor: SoundProcessorId) -> Option<egui::Rect> {
        self.processors.iter().find_map(|pp| {
            if pp.processor == processor {
                Some(pp.rect)
            } else {
                None
            }
        })
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
        self.processors.clear();
    }
}
