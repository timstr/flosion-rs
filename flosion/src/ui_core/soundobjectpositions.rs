use std::collections::HashMap;

use eframe::egui;
use hashstash::{Order, Stashable, Stasher, UnstashError, Unstashable, Unstasher};

use crate::core::sound::{
    expression::ProcessorExpressionLocation, soundgraph::SoundGraph,
    soundinput::SoundInputLocation, soundprocessor::SoundProcessorId,
};

use super::{
    interactions::draganddrop::DragDropSubject,
    stackedlayout::interconnect::{InputSocket, ProcessorPlug},
};

pub(crate) struct ProcessorPosition {
    /// The id of the processor
    pub(crate) processor: SoundProcessorId,

    // The on-screen area occupied by the processor's body
    pub(crate) body_rect: egui::Rect,

    // The on-screen area occupied by the processor and all of its inputs and sockets
    pub(crate) outer_rect: egui::Rect,
}

fn stash_rect(rect: egui::Rect, stasher: &mut Stasher) {
    stasher.f32(rect.left());
    stasher.f32(rect.right());
    stasher.f32(rect.top());
    stasher.f32(rect.bottom());
}

fn unstash_rect(unstasher: &mut Unstasher) -> Result<egui::Rect, UnstashError> {
    let left = unstasher.f32()?;
    let right = unstasher.f32()?;
    let top = unstasher.f32()?;
    let bottom = unstasher.f32()?;

    Ok(egui::Rect::from_x_y_ranges(left..=right, top..=bottom))
}

impl Stashable for ProcessorPosition {
    fn stash(&self, stasher: &mut Stasher<()>) {
        self.processor.stash(stasher);
        stash_rect(self.body_rect, stasher);
        stash_rect(self.outer_rect, stasher);
    }
}

impl Unstashable for ProcessorPosition {
    fn unstash(unstasher: &mut Unstasher<()>) -> Result<Self, UnstashError> {
        let processor = SoundProcessorId::unstash(unstasher)?;

        let body_rect = unstash_rect(unstasher)?;
        let outer_rect = unstash_rect(unstasher)?;

        Ok(ProcessorPosition {
            processor,
            body_rect,
            outer_rect,
        })
    }
}

pub(crate) struct SoundObjectPositions {
    socket_jumpers: HashMap<SoundInputLocation, egui::Rect>,
    processors: HashMap<SoundProcessorId, ProcessorPosition>,
    drag_drop_subjects: HashMap<DragDropSubject, egui::Rect>,
    expressions: HashMap<ProcessorExpressionLocation, egui::Rect>,
}

impl SoundObjectPositions {
    pub(crate) fn new() -> SoundObjectPositions {
        SoundObjectPositions {
            socket_jumpers: HashMap::new(),
            processors: HashMap::new(),
            drag_drop_subjects: HashMap::new(),
            expressions: HashMap::new(),
        }
    }

    pub(crate) fn socket_jumpers(&self) -> &HashMap<SoundInputLocation, egui::Rect> {
        &self.socket_jumpers
    }

    pub(crate) fn processors(&self) -> &HashMap<SoundProcessorId, ProcessorPosition> {
        &self.processors
    }

    pub(crate) fn drag_drop_subjects(&self) -> &HashMap<DragDropSubject, egui::Rect> {
        &self.drag_drop_subjects
    }

    pub(crate) fn expressions(&self) -> &HashMap<ProcessorExpressionLocation, egui::Rect> {
        &self.expressions
    }

    pub(crate) fn processor_expressions_top_down(
        &self,
        processor: SoundProcessorId,
    ) -> Vec<ProcessorExpressionLocation> {
        let mut v: Vec<(ProcessorExpressionLocation, i32)> = self
            .expressions
            .iter()
            .filter_map(|(k, v)| {
                if k.processor() == processor {
                    Some((*k, v.top().round() as i32))
                } else {
                    None
                }
            })
            .collect();

        v.sort_by(|a, b| a.1.cmp(&b.1));

        v.into_iter().map(|x| x.0).collect()
    }

    pub(crate) fn record_plug(&mut self, plug: ProcessorPlug, rect: egui::Rect) {
        self.drag_drop_subjects
            .insert(DragDropSubject::Plug(plug.processor), rect);
    }

    pub(crate) fn record_socket(&mut self, socket: InputSocket, rect: egui::Rect) {
        self.drag_drop_subjects
            .insert(DragDropSubject::Socket(socket.location), rect);
    }

    pub(crate) fn record_socket_jumper(
        &mut self,
        input_location: SoundInputLocation,
        rect: egui::Rect,
    ) {
        self.socket_jumpers.insert(input_location, rect);
    }

    pub(crate) fn record_expression(
        &mut self,
        expr_id: ProcessorExpressionLocation,
        rect: egui::Rect,
    ) {
        self.expressions.insert(expr_id, rect);
    }

    pub(crate) fn record_processor(
        &mut self,
        processor: SoundProcessorId,
        body_rect: egui::Rect,
        outer_rect: egui::Rect,
    ) {
        self.processors.insert(
            processor,
            ProcessorPosition {
                processor,
                body_rect,
                outer_rect,
            },
        );
        self.drag_drop_subjects
            .insert(DragDropSubject::Processor(processor), body_rect);
    }

    pub(crate) fn find_processor(&self, processor: SoundProcessorId) -> Option<&ProcessorPosition> {
        self.processors.get(&processor)
    }

    pub(crate) fn cleanup(&mut self, graph: &SoundGraph) {
        self.socket_jumpers.retain(|l, _| graph.contains(l));
        self.processors.retain(|x, _| graph.contains(x));
        self.drag_drop_subjects.retain(|x, _| x.is_valid(graph));
        self.expressions.retain(|x, _| graph.contains(x));
    }
}

impl Stashable for SoundObjectPositions {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.array_of_proxy_objects(
            self.socket_jumpers.iter(),
            |(k, v), stasher| {
                stasher.object(k);
                stash_rect(**v, stasher);
            },
            Order::Unordered,
        );
        stasher.array_of_proxy_objects(
            self.processors.iter(),
            |(k, v), stasher| {
                stasher.object(k);
                stasher.object(v);
            },
            Order::Unordered,
        );
        stasher.array_of_proxy_objects(
            self.drag_drop_subjects.iter(),
            |(k, v), stasher| {
                stasher.object(k);
                stash_rect(**v, stasher);
            },
            Order::Unordered,
        );
        stasher.array_of_proxy_objects(
            self.expressions.iter(),
            |(k, v), stasher| {
                stasher.object(k);
                stash_rect(**v, stasher);
            },
            Order::Unordered,
        );
    }
}

impl Unstashable for SoundObjectPositions {
    fn unstash(unstasher: &mut Unstasher) -> Result<Self, UnstashError> {
        let mut positions = SoundObjectPositions {
            socket_jumpers: HashMap::new(),
            processors: HashMap::new(),
            drag_drop_subjects: HashMap::new(),
            expressions: HashMap::new(),
        };

        unstasher.array_of_proxy_objects(|unstasher| {
            positions
                .socket_jumpers
                .insert(unstasher.object()?, unstash_rect(unstasher)?);
            Ok(())
        })?;
        unstasher.array_of_proxy_objects(|unstasher| {
            positions
                .processors
                .insert(unstasher.object()?, unstasher.object()?);
            Ok(())
        })?;
        unstasher.array_of_proxy_objects(|unstasher| {
            positions
                .drag_drop_subjects
                .insert(unstasher.object()?, unstash_rect(unstasher)?);
            Ok(())
        })?;
        unstasher.array_of_proxy_objects(|unstasher| {
            positions
                .expressions
                .insert(unstasher.object()?, unstash_rect(unstasher)?);
            Ok(())
        })?;

        Ok(positions)
    }
}
