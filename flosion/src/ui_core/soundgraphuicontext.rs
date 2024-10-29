use eframe::egui;
use hashstash::Stash;

use crate::core::jit::cache::JitCache;

use super::{
    factories::Factories, graph_properties::GraphProperties, stackedlayout::timeaxis::TimeAxis,
};

pub struct SoundGraphUiContext<'a, 'ctx> {
    factories: &'a Factories,
    time_axis: TimeAxis,
    width: f32,
    group_origin: egui::Pos2,
    properties: &'a GraphProperties,
    jit_cache: &'a JitCache<'ctx>,
    stash: &'a Stash,
}

impl<'a, 'ctx> SoundGraphUiContext<'a, 'ctx> {
    pub(crate) fn new(
        factories: &'a Factories,
        time_axis: TimeAxis,
        width: f32,
        group_origin: egui::Pos2,
        properties: &'a GraphProperties,
        jit_cache: &'a JitCache<'ctx>,
        stash: &'a Stash,
    ) -> SoundGraphUiContext<'a, 'ctx> {
        SoundGraphUiContext {
            factories,
            time_axis,
            width,
            group_origin,
            properties,
            jit_cache,
            stash,
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

    pub fn jit_cache(&self) -> &JitCache<'ctx> {
        self.jit_cache
    }

    pub fn stash(&self) -> &Stash {
        self.stash
    }
}
