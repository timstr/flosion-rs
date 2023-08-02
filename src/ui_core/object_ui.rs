use std::any::Any;

use eframe::{
    egui::{self},
    epaint::ecolor::{self},
};
use rand::{thread_rng, Rng};

use crate::core::{
    arguments::{ArgumentList, ParsedArguments},
    graph::graphobject::{GraphObjectHandle, ObjectHandle, ObjectInitialization},
    serialization::Serializable,
};

use super::{
    graph_ui::{GraphUi, ObjectUiData},
    object_ui_states::AnyObjectUiState,
};

impl ObjectUiState for () {}

pub fn random_object_color() -> egui::Color32 {
    let hue: f32 = thread_rng().gen();
    let color = ecolor::Hsva::new(hue, 1.0, 0.5, 1.0);
    color.into()
}

pub enum UiInitialization<'a> {
    Args(&'a ParsedArguments),
    Default,
}

pub trait ObjectUiState: Any + Default + Serializable {}

pub trait ObjectUi: 'static + Default {
    // TODO: find a way to clean up these darn nested types
    type GraphUi: GraphUi;
    type HandleType: ObjectHandle<<Self::GraphUi as GraphUi>::Graph>;
    type StateType: ObjectUiState;

    fn ui<'a, 'b>(
        &self,
        handle: Self::HandleType,
        graph_state: &mut <Self::GraphUi as GraphUi>::State,
        ui: &mut egui::Ui,
        ctx: &<Self::GraphUi as GraphUi>::Context<'a>,
        data: <<Self::GraphUi as GraphUi>::ObjectUiData as ObjectUiData>::ConcreteType<
            'b,
            Self::StateType,
        >,
    );

    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    fn arguments(&self) -> ArgumentList {
        ArgumentList::new()
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> Self::StateType {
        Self::StateType::default()
    }
}

pub trait AnyObjectUi<G: GraphUi> {
    fn apply(
        &self,
        object: &GraphObjectHandle<G::Graph>,
        object_ui_state: &mut G::ObjectUiData,
        graph_state: &mut G::State,
        ui: &mut egui::Ui,
        ctx: &G::Context<'_>,
    );

    fn aliases(&self) -> &'static [&'static str];

    fn arguments(&self) -> ArgumentList;

    fn make_ui_state(
        &self,
        object: &GraphObjectHandle<G::Graph>,
        init: ObjectInitialization,
    ) -> Result<Box<dyn AnyObjectUiState>, ()>;
}

impl<G: GraphUi, T: ObjectUi<GraphUi = G>> AnyObjectUi<G> for T {
    fn apply(
        &self,
        object: &GraphObjectHandle<G::Graph>,
        object_ui_state: &mut G::ObjectUiData,
        graph_state: &mut G::State,
        ui: &mut egui::Ui,
        ctx: &G::Context<'_>,
    ) {
        let handle = T::HandleType::from_graph_object(object.clone()).unwrap();
        let data = object_ui_state.downcast::<T::StateType>(graph_state, ctx);
        self.ui(handle, graph_state, ui, ctx, data);
    }

    fn aliases(&self) -> &'static [&'static str] {
        self.aliases()
    }

    fn arguments(&self) -> ArgumentList {
        self.arguments()
    }

    fn make_ui_state(
        &self,
        object: &GraphObjectHandle<G::Graph>,
        init: ObjectInitialization,
    ) -> Result<Box<dyn AnyObjectUiState>, ()> {
        // let dc_object = downcast_object_ref::<T>(object.instance());
        let handle = T::HandleType::from_graph_object(object.clone()).unwrap();
        let state: T::StateType = match init {
            ObjectInitialization::Args(a) => self.make_ui_state(&handle, UiInitialization::Args(a)),
            ObjectInitialization::Archive(mut a) => T::StateType::deserialize(&mut a)?,
            ObjectInitialization::Default => self.make_ui_state(&handle, UiInitialization::Default),
        };
        Ok(Box::new(state))
    }
}
