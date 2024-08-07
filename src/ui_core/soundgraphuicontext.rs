use std::collections::{HashMap, HashSet};

use eframe::egui;

use crate::core::sound::{
    expression::SoundExpressionId, expressionargument::SoundExpressionArgumentId,
};

use super::{flosion_ui::Factories, stackedlayout::timeaxis::TimeAxis};

pub struct SoundGraphUiContext<'a> {
    factories: &'a Factories,
    time_axis: TimeAxis,
    width: f32,
    group_origin: egui::Pos2,
    available_arguments: &'a HashMap<SoundExpressionId, HashSet<SoundExpressionArgumentId>>,
}

impl<'a> SoundGraphUiContext<'a> {
    pub(crate) fn new(
        factories: &'a Factories,
        time_axis: TimeAxis,
        width: f32,
        group_origin: egui::Pos2,
        available_arguments: &'a HashMap<SoundExpressionId, HashSet<SoundExpressionArgumentId>>,
    ) -> SoundGraphUiContext<'a> {
        SoundGraphUiContext {
            factories,
            time_axis,
            width,
            group_origin,
            available_arguments,
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

    pub fn available_arguments(
        &self,
    ) -> &HashMap<SoundExpressionId, HashSet<SoundExpressionArgumentId>> {
        self.available_arguments
    }
}
