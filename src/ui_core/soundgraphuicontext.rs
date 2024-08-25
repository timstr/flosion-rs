use eframe::egui;

use super::{
    flosion_ui::Factories, graph_properties::GraphProperties, stackedlayout::timeaxis::TimeAxis,
};

pub struct SoundGraphUiContext<'a> {
    factories: &'a Factories,
    time_axis: TimeAxis,
    width: f32,
    group_origin: egui::Pos2,
    properties: &'a GraphProperties,
}

impl<'a> SoundGraphUiContext<'a> {
    pub(crate) fn new(
        factories: &'a Factories,
        time_axis: TimeAxis,
        width: f32,
        group_origin: egui::Pos2,
        properties: &'a GraphProperties,
    ) -> SoundGraphUiContext<'a> {
        SoundGraphUiContext {
            factories,
            time_axis,
            width,
            group_origin,
            properties,
        }
    }

    pub(crate) fn factories(&self) -> &Factories {
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
}
