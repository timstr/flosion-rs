use chive::{Chivable, ChiveIn, ChiveOut};
use eframe::{
    egui::{self},
    epaint::ecolor::{self},
};
use rand::{thread_rng, Rng};

use crate::core::graph::{
    graph::Graph,
    graphobject::{GraphObjectHandle, ObjectHandle, ObjectInitialization, ObjectType},
};

use super::{
    arguments::{ArgumentList, ParsedArguments},
    graph_ui::{GraphUi, ObjectUiData, ObjectUiState},
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

impl ObjectUiState for () {}

pub fn random_object_color() -> egui::Color32 {
    let hue: f32 = thread_rng().gen();
    let color = ecolor::Hsva::new(hue, 1.0, 0.5, 1.0);
    color.into()
}

pub enum UiInitialization {
    Default,
    Arguments(ParsedArguments),
}

pub trait ObjectUi: 'static + Default {
    // TODO: find a way to clean up these darn nested types
    type GraphUi: GraphUi;
    type HandleType: ObjectHandle<<Self::GraphUi as GraphUi>::Graph>;
    type StateType: ObjectUiState;

    fn ui<'a>(
        &self,
        handle: Self::HandleType,
        ui_state: &mut <Self::GraphUi as GraphUi>::State,
        ui: &mut egui::Ui,
        ctx: &<Self::GraphUi as GraphUi>::Context<'_>,
        data: <<Self::GraphUi as GraphUi>::ObjectUiData as ObjectUiData>::ConcreteType<
            'a,
            Self::StateType,
        >,
        graph: &mut <Self::GraphUi as GraphUi>::Graph,
    );

    fn summon_names(&self) -> &'static [&'static str];

    fn summon_arguments(&self) -> ArgumentList {
        ArgumentList::new_empty()
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (
        Self::StateType,
        <<Self::GraphUi as GraphUi>::ObjectUiData as ObjectUiData>::RequiredData,
    );
}

pub trait AnyObjectUi<G: GraphUi> {
    fn apply(
        &self,
        object: &GraphObjectHandle<G::Graph>,
        object_ui_state: &G::ObjectUiData,
        graph_state: &mut G::State,
        ui: &mut egui::Ui,
        ctx: &G::Context<'_>,
        graph: &mut G::Graph,
    );

    fn summon_names(&self) -> &'static [&'static str];

    fn summon_arguments(&self) -> ArgumentList;

    fn object_type(&self) -> ObjectType;

    fn make_ui_state(
        &self,
        id: <G::Graph as Graph>::ObjectId,
        object: &GraphObjectHandle<G::Graph>,
        init: ObjectInitialization,
    ) -> Result<G::ObjectUiData, ()>;
}

impl<G: GraphUi, T: ObjectUi<GraphUi = G>> AnyObjectUi<G> for T {
    fn apply(
        &self,
        object: &GraphObjectHandle<G::Graph>,
        object_ui_state: &G::ObjectUiData,
        graph_state: &mut G::State,
        ui: &mut egui::Ui,
        ctx: &G::Context<'_>,
        graph: &mut G::Graph,
    ) {
        let handle = T::HandleType::from_graph_object(object.clone()).unwrap();
        object_ui_state.downcast_with(graph_state, ctx, |data, graph_state, ctx| {
            self.ui(handle, graph_state, ui, ctx, data, graph);
        });
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

    fn make_ui_state(
        &self,
        id: <G::Graph as Graph>::ObjectId,
        object: &GraphObjectHandle<G::Graph>,
        init: ObjectInitialization,
    ) -> Result<G::ObjectUiData, ()> {
        let handle = T::HandleType::from_graph_object(object.clone()).unwrap();
        let (state, required_data) = match init {
            ObjectInitialization::Deserialize(mut a) => {
                (Chivable::chive_out(&mut a)?, Chivable::chive_out(&mut a)?)
            }
            ObjectInitialization::Default => self.make_ui_state(&handle, UiInitialization::Default),
            ObjectInitialization::Arguments(parsed_args) => {
                self.make_ui_state(&handle, UiInitialization::Arguments(parsed_args))
            }
        };

        Ok(<G::ObjectUiData as ObjectUiData>::new(
            id,
            state,
            required_data,
        ))
    }
}
