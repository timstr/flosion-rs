use eframe::{
    egui::{self},
    epaint::ecolor::{self},
};
use rand::{thread_rng, Rng};
use serialization::{Deserializer, Serializable, Serializer};

use crate::core::graph::{
    graph::Graph,
    graphobject::{GraphObjectHandle, ObjectHandle, ObjectInitialization},
};

use super::graph_ui::{GraphUi, ObjectUiData, ObjectUiState};

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

impl Serializable for Color {
    fn serialize(&self, serializer: &mut Serializer) {
        serializer.u32(u32::from_be_bytes(self.color.to_array()))
    }

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, ()> {
        let i = deserializer.u32()?;
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
}

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
        ctx: &mut <Self::GraphUi as GraphUi>::Context<'a>,
        data: <<Self::GraphUi as GraphUi>::ObjectUiData as ObjectUiData>::ConcreteType<
            'b,
            Self::StateType,
        >,
    );

    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (
        Self::StateType,
        <<Self::GraphUi as GraphUi>::ObjectUiData as ObjectUiData>::RequiredData,
    ) {
        (Default::default(), Default::default())
    }
}

pub trait AnyObjectUi<G: GraphUi> {
    fn apply(
        &self,
        object: &GraphObjectHandle<G::Graph>,
        object_ui_state: &G::ObjectUiData,
        graph_state: &mut G::State,
        ui: &mut egui::Ui,
        ctx: &mut G::Context<'_>,
    );

    fn aliases(&self) -> &'static [&'static str];

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
        ctx: &mut G::Context<'_>,
    ) {
        let handle = T::HandleType::from_graph_object(object.clone()).unwrap();
        object_ui_state.downcast_with(graph_state, ctx, |data, graph_state, ctx| {
            self.ui(handle, graph_state, ui, ctx, data);
        });
    }

    fn aliases(&self) -> &'static [&'static str] {
        self.aliases()
    }

    fn make_ui_state(
        &self,
        id: <G::Graph as Graph>::ObjectId,
        object: &GraphObjectHandle<G::Graph>,
        init: ObjectInitialization,
    ) -> Result<G::ObjectUiData, ()> {
        let handle = T::HandleType::from_graph_object(object.clone()).unwrap();
        let (state, required_data) = match init {
            ObjectInitialization::Archive(mut a) => (
                T::StateType::deserialize(&mut a)?,
                Serializable::deserialize(&mut a)?,
            ),
            ObjectInitialization::Default => self.make_ui_state(&handle, UiInitialization::Default),
        };

        Ok(<G::ObjectUiData as ObjectUiData>::new(
            id,
            state,
            required_data,
        ))
    }
}
