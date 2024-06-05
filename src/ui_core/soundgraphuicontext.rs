use super::{flosion_ui::Factories, soundgraphlayout::TimeAxis};

pub struct SoundGraphUiContext<'a> {
    factories: &'a Factories,
    time_axis: TimeAxis,
    width: f32,
}

impl<'a> SoundGraphUiContext<'a> {
    pub(crate) fn new(
        factories: &'a Factories,
        time_axis: TimeAxis,
        width: f32,
    ) -> SoundGraphUiContext<'a> {
        SoundGraphUiContext {
            factories,
            time_axis,
            width,
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
}
