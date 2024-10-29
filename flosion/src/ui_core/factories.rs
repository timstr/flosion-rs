use crate::{
    core::{
        expression::expressionobject::ExpressionObjectFactory,
        sound::soundobject::SoundObjectFactory,
    },
    ui_objects::all_objects::{all_expression_graph_objects, all_sound_graph_objects},
};

use super::{expressionobjectui::ExpressionObjectUiFactory, soundobjectui::SoundObjectUiFactory};

/// Convenience struct for passing all the different factories together
pub(crate) struct Factories {
    sound_objects: SoundObjectFactory,
    expression_objects: ExpressionObjectFactory,
    sound_uis: SoundObjectUiFactory,
    expression_uis: ExpressionObjectUiFactory,
}

impl Factories {
    pub(crate) fn new_empty() -> Factories {
        Factories {
            sound_objects: SoundObjectFactory::new_empty(),
            expression_objects: ExpressionObjectFactory::new_empty(),
            sound_uis: SoundObjectUiFactory::new_empty(),
            expression_uis: ExpressionObjectUiFactory::new_empty(),
        }
    }

    /// Creates a new set of factories pre-filled with all statically registered types
    pub(crate) fn new_all_objects() -> Factories {
        let (object_factory, ui_factory) = all_sound_graph_objects();
        let (expression_object_factory, expression_ui_factory) = all_expression_graph_objects();

        Factories {
            sound_objects: object_factory,
            expression_objects: expression_object_factory,
            sound_uis: ui_factory,
            expression_uis: expression_ui_factory,
        }
    }

    pub(crate) fn sound_objects(&self) -> &SoundObjectFactory {
        &self.sound_objects
    }

    pub(crate) fn sound_objects_mut(&mut self) -> &mut SoundObjectFactory {
        &mut self.sound_objects
    }

    pub(crate) fn expression_objects(&self) -> &ExpressionObjectFactory {
        &self.expression_objects
    }

    pub(crate) fn expression_objects_mut(&mut self) -> &mut ExpressionObjectFactory {
        &mut self.expression_objects
    }

    pub(crate) fn sound_uis(&self) -> &SoundObjectUiFactory {
        &self.sound_uis
    }

    pub(crate) fn sound_uis_mut(&mut self) -> &mut SoundObjectUiFactory {
        &mut self.sound_uis
    }

    pub(crate) fn expression_uis(&self) -> &ExpressionObjectUiFactory {
        &self.expression_uis
    }

    pub(crate) fn expression_uis_mut(&mut self) -> &mut ExpressionObjectUiFactory {
        &mut self.expression_uis
    }
}
