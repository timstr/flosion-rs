use std::{any::Any, cell::RefCell, rc::Rc};

use chive::{Chivable, ChiveIn, ChiveOut};
use eframe::{
    egui::{self},
    epaint::ecolor::{self},
};
use rand::{thread_rng, Rng};

use crate::core::graph::graphobject::{GraphObjectHandle, ObjectHandle, ObjectType};

use super::{
    arguments::{ArgumentList, ParsedArguments},
    graph_ui::GraphUi,
};

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

impl Chivable for Color {
    fn chive_in(&self, chive_in: &mut ChiveIn) {
        chive_in.u32(u32::from_be_bytes(self.color.to_array()))
    }

    fn chive_out(chive_out: &mut ChiveOut) -> Result<Self, ()> {
        let i = chive_out.u32()?;
        let [r, g, b, a] = i.to_be_bytes();
        Ok(Color {
            color: egui::Color32::from_rgba_premultiplied(r, g, b, a),
        })
    }
}

pub fn random_object_color() -> egui::Color32 {
    let hue: f32 = thread_rng().gen();
    let color = ecolor::Hsva::new(hue, 1.0, 0.5, 1.0);
    color.into()
}

pub trait ObjectUi: 'static + Default {
    // TODO: find a way to clean up these darn nested types
    type GraphUi: GraphUi;
    type HandleType: ObjectHandle<<Self::GraphUi as GraphUi>::Graph>;
    type StateType;

    fn ui<'a>(
        &self,
        handle: Self::HandleType,
        graph_ui_state: &mut <Self::GraphUi as GraphUi>::State,
        ui: &mut egui::Ui,
        ctx: &<Self::GraphUi as GraphUi>::Context<'_>,
        state: &mut Self::StateType,
        graph: &mut <Self::GraphUi as GraphUi>::Graph,
    );

    fn summon_names(&self) -> &'static [&'static str];

    fn summon_arguments(&self) -> ArgumentList {
        ArgumentList::new_empty()
    }

    fn make_properties(&self) -> <Self::GraphUi as GraphUi>::Properties;

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _args: ParsedArguments,
    ) -> Result<Self::StateType, ()>;
}

pub trait AnyObjectUi<G: GraphUi> {
    fn apply(
        &self,
        object: &GraphObjectHandle<G::Graph>,
        state: &mut dyn Any,
        graph_state: &mut G::State,
        ui: &mut egui::Ui,
        ctx: &G::Context<'_>,
        graph: &mut G::Graph,
    );

    fn summon_names(&self) -> &'static [&'static str];

    fn summon_arguments(&self) -> ArgumentList;

    fn object_type(&self) -> ObjectType;

    fn make_properties(&self) -> G::Properties;

    fn make_ui_state(
        &self,
        object: &GraphObjectHandle<G::Graph>,
        args: ParsedArguments,
    ) -> Result<Rc<RefCell<dyn Any>>, ()>;
}

impl<G: GraphUi, T: ObjectUi<GraphUi = G>> AnyObjectUi<G> for T {
    fn apply(
        &self,
        object: &GraphObjectHandle<G::Graph>,
        state: &mut dyn Any,
        graph_state: &mut G::State,
        ui: &mut egui::Ui,
        ctx: &G::Context<'_>,
        graph: &mut G::Graph,
    ) {
        let handle = T::HandleType::from_graph_object(object.clone()).unwrap();
        self.ui(
            handle,
            graph_state,
            ui,
            ctx,
            state.downcast_mut().unwrap(),
            graph,
        );
    }

    fn summon_names(&self) -> &'static [&'static str] {
        self.summon_names()
    }

    fn summon_arguments(&self) -> ArgumentList {
        T::summon_arguments(self)
    }

    fn object_type(&self) -> ObjectType {
        <T::HandleType as ObjectHandle<G::Graph>>::object_type()
    }

    fn make_properties(&self) -> G::Properties {
        T::make_properties(&self)
    }

    fn make_ui_state(
        &self,
        object: &GraphObjectHandle<G::Graph>,
        args: ParsedArguments,
    ) -> Result<Rc<RefCell<dyn Any>>, ()> {
        let handle = T::HandleType::from_graph_object(object.clone()).unwrap();
        let state = self.make_ui_state(&handle, args)?;
        Ok(Rc::new(RefCell::new(state)))
    }
}
