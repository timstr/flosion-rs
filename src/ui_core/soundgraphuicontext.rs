use std::collections::{HashMap, HashSet};

use crate::core::sound::{
    expression::SoundExpressionId, expressionargument::SoundExpressionArgumentId,
};

use super::{flosion_ui::Factories, soundgraphlayout::TimeAxis};

pub struct SoundGraphUiContext<'a> {
    factories: &'a Factories,
    time_axis: TimeAxis,
    width: f32,
    available_arguments: &'a HashMap<SoundExpressionId, HashSet<SoundExpressionArgumentId>>,
}

impl<'a> SoundGraphUiContext<'a> {
    pub(crate) fn new(
        factories: &'a Factories,
        time_axis: TimeAxis,
        width: f32,
        available_arguments: &'a HashMap<SoundExpressionId, HashSet<SoundExpressionArgumentId>>,
    ) -> SoundGraphUiContext<'a> {
        SoundGraphUiContext {
            factories,
            time_axis,
            width,
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

    pub fn available_arguments(
        &self,
    ) -> &HashMap<SoundExpressionId, HashSet<SoundExpressionArgumentId>> {
        self.available_arguments
    }
}
