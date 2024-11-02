use std::any::Any;

use eframe::{
    egui::{self},
    epaint::ecolor::{self},
};
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};
use rand::{thread_rng, Rng};

// TODO: this module is misnamed now (any maybe pointless?)

pub struct Color {
    pub color: egui::Color32,
}

impl Default for Color {
    fn default() -> Self {
        Color {
            color: random_object_color(),
        }
    }
}

pub fn random_object_color() -> egui::Color32 {
    let hue: f32 = thread_rng().gen();
    let color = ecolor::Hsva::new(hue, 1.0, 0.5, 1.0);
    color.into()
}

pub trait ObjectUiState {
    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;

    fn stash(&self, stasher: &mut Stasher);

    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError>;
}

impl<T> ObjectUiState for T
where
    T: 'static + Stashable + UnstashableInplace,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn stash(&self, stasher: &mut Stasher) {
        T::stash(self, stasher);
    }

    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        T::unstash_inplace(self, unstasher)
    }
}

pub struct NoObjectUiState;

impl Stashable for NoObjectUiState {
    fn stash(&self, _stasher: &mut Stasher) {}
}

impl UnstashableInplace for NoObjectUiState {
    fn unstash_inplace(&mut self, _unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        Ok(())
    }
}
