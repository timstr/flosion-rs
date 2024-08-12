use eframe::egui;

use crate::core::sound::{soundinput::SoundInputId, soundprocessor::SoundProcessorId};

use super::stackedlayout::interconnect::{InputSocket, ProcessorPlug};

pub(crate) struct PositionedItems<T> {
    values: Vec<T>,
    positions: Vec<egui::Rect>,
}

impl<T> PositionedItems<T> {
    pub(crate) fn new() -> PositionedItems<T> {
        PositionedItems {
            values: Vec::new(),
            positions: Vec::new(),
        }
    }

    pub(crate) fn push(&mut self, item: T, rect: egui::Rect) {
        self.values.push(item);
        self.positions.push(rect);
    }

    pub(crate) fn clear(&mut self) {
        self.values.clear();
        self.positions.clear();
    }

    pub(crate) fn items(&self) -> impl Iterator<Item = (&T, egui::Rect)> {
        self.values.iter().zip(self.positions.iter().cloned())
    }

    pub(crate) fn values(&self) -> &[T] {
        &self.values
    }

    pub(crate) fn position(&self, item: &T) -> Option<egui::Rect>
    where
        T: PartialEq,
    {
        debug_assert_eq!(self.values.len(), self.positions.len());
        self.values
            .iter()
            .position(|i| i == item)
            .map(|idx| self.positions[idx])
    }

    pub(crate) fn find_position<P>(&self, predicate: P) -> Option<egui::Rect>
    where
        P: FnMut(&T) -> bool,
    {
        debug_assert_eq!(self.values.len(), self.positions.len());
        self.values
            .iter()
            .position(predicate)
            .map(|idx| self.positions[idx])
    }

    pub(crate) fn find_closest_where<F>(
        &self,
        query: egui::Rect,
        minimum_overlap_area: f32,
        mut f: F,
    ) -> Option<(&T, f32)>
    where
        F: FnMut(&T) -> bool,
    {
        let mut best_overlap = minimum_overlap_area;
        let mut best_index = None;
        for (index, rect) in self.positions.iter().enumerate() {
            if !f(&self.values[index]) {
                continue;
            }
            let intersection = rect.intersect(query);
            if !intersection.is_positive() {
                continue;
            }
            let area = intersection.area();
            if area > best_overlap {
                best_overlap = area;
                best_index = Some(index);
            }
        }
        best_index.map(|idx| (&self.values[idx], best_overlap))
    }
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
    plugs: PositionedItems<ProcessorPlug>,
    sockets: PositionedItems<InputSocket>,
    socket_jumpers: PositionedItems<SoundInputId>,
    processors: Vec<ProcessorPosition>,
}

impl SoundObjectPositions {
    pub(crate) fn new() -> SoundObjectPositions {
        SoundObjectPositions {
            plugs: PositionedItems::new(),
            sockets: PositionedItems::new(),
            socket_jumpers: PositionedItems::new(),
            processors: Vec::new(),
        }
    }

    pub(crate) fn plugs(&self) -> &PositionedItems<ProcessorPlug> {
        &self.plugs
    }

    pub(crate) fn sockets(&self) -> &PositionedItems<InputSocket> {
        &self.sockets
    }

    pub(crate) fn socket_jumpers(&self) -> &PositionedItems<SoundInputId> {
        &self.socket_jumpers
    }

    pub(crate) fn processors(&self) -> &[ProcessorPosition] {
        &self.processors
    }

    pub(crate) fn record_plug(&mut self, plug: ProcessorPlug, rect: egui::Rect) {
        self.plugs.push(plug, rect);
    }

    pub(crate) fn record_socket(&mut self, socket: InputSocket, rect: egui::Rect) {
        self.sockets.push(socket, rect);
    }

    pub(crate) fn record_socket_jumper(&mut self, input_id: SoundInputId, rect: egui::Rect) {
        self.socket_jumpers.push(input_id, rect);
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

    pub(crate) fn find_processor(&self, processor: SoundProcessorId) -> Option<&ProcessorPosition> {
        self.processors.iter().find(|pp| pp.processor == processor)
    }

    pub(crate) fn clear(&mut self) {
        self.plugs.clear();
        self.sockets.clear();
        self.socket_jumpers.clear();
        self.processors.clear();
    }
}
