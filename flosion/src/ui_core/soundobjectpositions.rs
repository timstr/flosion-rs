use eframe::egui;
use hashstash::{Order, Stashable, Stasher, UnstashError, Unstashable, Unstasher};

use crate::core::sound::{
    expression::ProcessorExpressionLocation, soundinput::SoundInputLocation,
    soundprocessor::SoundProcessorId,
};

use super::{
    interactions::draganddrop::DragDropSubject,
    stackedlayout::interconnect::{InputSocket, ProcessorPlug},
};

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
    ) -> Option<&T>
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
        best_index.map(|idx| &self.values[idx])
    }
}

impl<T: Stashable> Stashable for PositionedItems<T> {
    fn stash(&self, stasher: &mut Stasher<()>) {
        stasher.array_of_proxy_objects(
            self.values.iter().zip(&self.positions),
            |(value, position), stasher| {
                value.stash(stasher);
                stasher.f32(position.left());
                stasher.f32(position.right());
                stasher.f32(position.top());
                stasher.f32(position.bottom());
            },
            Order::Unordered,
        );
    }
}

impl<T: Unstashable> Unstashable for PositionedItems<T> {
    fn unstash(unstasher: &mut Unstasher<()>) -> Result<Self, UnstashError> {
        let mut values = Vec::new();
        let mut positions = Vec::new();
        unstasher.array_of_proxy_objects(|unstasher| {
            values.push(T::unstash(unstasher)?);

            let left = unstasher.f32()?;
            let right = unstasher.f32()?;
            let top = unstasher.f32()?;
            let bottom = unstasher.f32()?;

            positions.push(egui::Rect::from_x_y_ranges(left..=right, top..=bottom));

            Ok(())
        })?;

        Ok(PositionedItems { values, positions })
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

impl Stashable for ProcessorPosition {
    fn stash(&self, stasher: &mut Stasher<()>) {
        self.processor.stash(stasher);
        stasher.f32(self.rect.left());
        stasher.f32(self.rect.right());
        stasher.f32(self.rect.top());
        stasher.f32(self.rect.bottom());
        stasher.f32(self.group_origin.x);
        stasher.f32(self.group_origin.y);
    }
}

impl Unstashable for ProcessorPosition {
    fn unstash(unstasher: &mut Unstasher<()>) -> Result<Self, UnstashError> {
        let processor = SoundProcessorId::unstash(unstasher)?;

        let left = unstasher.f32()?;
        let right = unstasher.f32()?;
        let top = unstasher.f32()?;
        let bottom = unstasher.f32()?;

        let rect = egui::Rect::from_x_y_ranges(left..=right, top..=bottom);

        let group_origin = egui::pos2(unstasher.f32()?, unstasher.f32()?);

        Ok(ProcessorPosition {
            processor,
            rect,
            group_origin,
        })
    }
}

pub(crate) struct SoundObjectPositions {
    socket_jumpers: PositionedItems<SoundInputLocation>,
    processors: Vec<ProcessorPosition>,
    drag_drop_subjects: PositionedItems<DragDropSubject>,
    expressions: PositionedItems<ProcessorExpressionLocation>,
}

impl SoundObjectPositions {
    pub(crate) fn new() -> SoundObjectPositions {
        SoundObjectPositions {
            socket_jumpers: PositionedItems::new(),
            processors: Vec::new(),
            drag_drop_subjects: PositionedItems::new(),
            expressions: PositionedItems::new(),
        }
    }

    pub(crate) fn socket_jumpers(&self) -> &PositionedItems<SoundInputLocation> {
        &self.socket_jumpers
    }

    pub(crate) fn processors(&self) -> &[ProcessorPosition] {
        &self.processors
    }

    pub(crate) fn drag_drop_subjects(&self) -> &PositionedItems<DragDropSubject> {
        &self.drag_drop_subjects
    }

    pub(crate) fn expressions(&self) -> &PositionedItems<ProcessorExpressionLocation> {
        &self.expressions
    }

    pub(crate) fn record_plug(&mut self, plug: ProcessorPlug, rect: egui::Rect) {
        self.drag_drop_subjects
            .push(DragDropSubject::Plug(plug.processor), rect);
    }

    pub(crate) fn record_socket(&mut self, socket: InputSocket, rect: egui::Rect) {
        self.drag_drop_subjects
            .push(DragDropSubject::Socket(socket.location), rect);
    }

    pub(crate) fn record_socket_jumper(
        &mut self,
        input_location: SoundInputLocation,
        rect: egui::Rect,
    ) {
        self.socket_jumpers.push(input_location, rect);
    }

    pub(crate) fn record_expression(
        &mut self,
        expr_id: ProcessorExpressionLocation,
        rect: egui::Rect,
    ) {
        self.expressions.push(expr_id, rect);
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
        self.drag_drop_subjects
            .push(DragDropSubject::Processor(processor), rect);
    }

    pub(crate) fn find_processor(&self, processor: SoundProcessorId) -> Option<&ProcessorPosition> {
        self.processors.iter().find(|pp| pp.processor == processor)
    }

    pub(crate) fn clear(&mut self) {
        self.socket_jumpers.clear();
        self.processors.clear();
        self.drag_drop_subjects.clear();
        self.expressions.clear();
    }
}

impl Stashable for SoundObjectPositions {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.object(&self.socket_jumpers);
        stasher.array_of_objects_slice(&self.processors, Order::Unordered);
        stasher.object(&self.drag_drop_subjects);
        stasher.object(&self.expressions);
    }
}

impl Unstashable for SoundObjectPositions {
    fn unstash(unstasher: &mut Unstasher) -> Result<Self, UnstashError> {
        Ok(SoundObjectPositions {
            socket_jumpers: unstasher.object()?,
            processors: unstasher.array_of_objects_vec()?,
            drag_drop_subjects: unstasher.object()?,
            expressions: unstasher.object()?,
        })
    }
}
