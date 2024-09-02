use eframe::egui;

use crate::core::jit::server::JitServer;

use super::{
    flosion_ui::Factories, graph_properties::GraphProperties, stackedlayout::timeaxis::TimeAxis,
};

pub struct SoundGraphUiContext<'a> {
    factories: &'a Factories,
    time_axis: TimeAxis,
    width: f32,
    group_origin: egui::Pos2,
    properties: &'a GraphProperties,
    jit_server: &'a JitServer<'a>,
}

impl<'a> SoundGraphUiContext<'a> {
    pub(crate) fn new(
        factories: &'a Factories,
        time_axis: TimeAxis,
        width: f32,
        group_origin: egui::Pos2,
        properties: &'a GraphProperties,
        jit_server: &'a JitServer<'a>,
    ) -> SoundGraphUiContext<'a> {
        SoundGraphUiContext {
            factories,
            time_axis,
            width,
            group_origin,
            properties,
            jit_server,
        }
    }

    pub(crate) fn factories(&self) -> &'a Factories {
        self.factories
    }

    pub fn time_axis(&self) -> &TimeAxis {
        &self.time_axis
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn group_origin(&self) -> egui::Pos2 {
        self.group_origin
    }

    pub fn properties(&self) -> &GraphProperties {
        self.properties
    }

    pub fn jit_server(&self) -> &'a JitServer<'a> {
        self.jit_server
    }
}
