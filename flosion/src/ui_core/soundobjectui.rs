use std::{cell::RefCell, collections::HashMap, ops::Deref, rc::Rc};

use eframe::egui;

use crate::core::{objecttype::ObjectType, sound::soundobject::SoundGraphObject};

use super::{
    arguments::{ArgumentList, ParsedArguments},
    object_ui::ObjectUiState,
    soundgraphuicontext::SoundGraphUiContext,
    soundgraphuistate::SoundGraphUiState,
};

pub trait SoundObjectUi: Default {
    type ObjectType: SoundGraphObject;
    type StateType: ObjectUiState;

    fn ui<'a>(
        &self,
        object: &mut Self::ObjectType,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        state: &mut Self::StateType,
    );

    fn summon_names(&self) -> &'static [&'static str];

    fn summon_arguments(&self) -> ArgumentList {
        ArgumentList::new_empty()
    }

    // TODO: remove
    fn make_properties(&self) -> ();

    fn make_ui_state(
        &self,
        _object: &Self::ObjectType,
        _args: &ParsedArguments,
    ) -> Result<Self::StateType, ()>;
}

pub trait AnySoundObjectUi {
    fn apply(
        &self,
        object: &mut dyn SoundGraphObject,
        state: &mut dyn ObjectUiState,
        graph_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
    );

    fn summon_names(&self) -> &'static [&'static str];

    fn summon_arguments(&self) -> ArgumentList;

    fn object_type(&self) -> ObjectType;

    // TODO: remove
    fn make_properties(&self) -> ();

    fn make_ui_state(
        &self,
        object: &dyn SoundGraphObject,
        args: &ParsedArguments,
    ) -> Result<Rc<RefCell<dyn ObjectUiState>>, ()>;
}

impl<T: 'static + SoundObjectUi> AnySoundObjectUi for T {
    fn apply(
        &self,
        object: &mut dyn SoundGraphObject,
        state: &mut dyn ObjectUiState,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
    ) {
        let object = object.as_mut_any().downcast_mut::<T::ObjectType>().unwrap();
        self.ui(
            object,
            graph_ui_state,
            ui,
            ctx,
            state.as_mut_any().downcast_mut().unwrap(),
        );
    }

    fn summon_names(&self) -> &'static [&'static str] {
        self.summon_names()
    }

    fn summon_arguments(&self) -> ArgumentList {
        T::summon_arguments(self)
    }

    fn object_type(&self) -> ObjectType {
        <T::ObjectType as SoundGraphObject>::get_type()
    }

    // TODO: remove
    fn make_properties(&self) -> () {
        T::make_properties(&self)
    }

    fn make_ui_state(
        &self,
        object: &dyn SoundGraphObject,
        args: &ParsedArguments,
    ) -> Result<Rc<RefCell<dyn ObjectUiState>>, ()> {
        let object = object.as_any().downcast_ref::<T::ObjectType>().unwrap();
        let state = self.make_ui_state(object, args)?;
        Ok(Rc::new(RefCell::new(state)))
    }
}

pub(crate) struct SoundObjectUiFactory {
    mapping: HashMap<ObjectType, Box<dyn AnySoundObjectUi>>,
}

impl SoundObjectUiFactory {
    pub(crate) fn new_empty() -> SoundObjectUiFactory {
        SoundObjectUiFactory {
            mapping: HashMap::new(),
        }
    }

    pub(crate) fn register<T: 'static + SoundObjectUi>(&mut self) {
        let instance = T::default();
        let object_type = instance.object_type();
        self.mapping.insert(object_type, Box::new(instance));
    }

    pub(crate) fn get(&self, object_type: ObjectType) -> &dyn AnySoundObjectUi {
        self.mapping
            .get(&object_type)
            .unwrap_or_else(|| {
                panic!(
                    "Tried to create a ui for an sound graph object of unrecognized type \"{}\"",
                    object_type.name(),
                )
            })
            .deref()
    }

    pub(crate) fn all_object_uis(&self) -> impl Iterator<Item = &dyn AnySoundObjectUi> {
        self.mapping.values().map(|b| b.deref())
    }
}

pub(crate) fn show_sound_object_ui(
    factory: &SoundObjectUiFactory,
    object: &mut dyn SoundGraphObject,
    ui_state: &mut SoundGraphUiState,
    ui: &mut egui::Ui,
    ctx: &SoundGraphUiContext,
) {
    let object_type = object.get_dynamic_type();

    let object_ui = factory.get(object_type);

    let state = ui_state.object_states().get_object_data(object.id());
    let state: &mut dyn ObjectUiState = &mut *state.borrow_mut();
    object_ui.apply(object, state, ui_state, ui, ctx);
}
