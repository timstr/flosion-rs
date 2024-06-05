use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[cfg(debug_assertions)]
use std::any::type_name;

use crate::core::{
    expression::{
        expressiongraphtopology::ExpressionGraphTopology, expressionnode::ExpressionNodeId,
    },
    sound::{expression::SoundExpressionId, soundgraphtopology::SoundGraphTopology},
};

use super::{
    expressiongraphui::ExpressionGraphUi,
    expressiongraphuicontext::ExpressionGraphUiContext,
    expressionui::ExpressionPresentation,
    graph_ui::{GraphUiState, ObjectUiData, ObjectUiState},
    lexicallayout::lexicallayout::ExpressionNodeLayout,
    object_ui_states::AnyObjectUiState,
    ui_factory::UiFactory,
};

pub(super) struct ExpressionUiCollection {
    data: HashMap<SoundExpressionId, ExpressionPresentation>,
}

impl ExpressionUiCollection {
    pub(super) fn new() -> ExpressionUiCollection {
        ExpressionUiCollection {
            data: HashMap::new(),
        }
    }

    pub(super) fn set_ui_data(
        &mut self,
        niid: SoundExpressionId,
        presentation: ExpressionPresentation,
    ) {
        self.data.insert(niid, presentation);
    }

    pub(super) fn cleanup(
        &mut self,
        topology: &SoundGraphTopology,
        factory: &UiFactory<ExpressionGraphUi>,
    ) {
        // Delete data for removed expressions
        self.data
            .retain(|id, _| topology.expressions().contains_key(id));

        // Clean up the internal ui data of individual expressions
        for (niid, presentation) in &mut self.data {
            let number_topo = topology
                .expression(*niid)
                .unwrap()
                .expression_graph()
                .topology();
            presentation.cleanup(number_topo);
        }

        // Add data for newly-added expressions
        for expr in topology.expressions().values() {
            if self.data.contains_key(&expr.id()) {
                continue;
            }

            let ui_state = ();

            let presentation =
                ExpressionPresentation::new(expr.expression_graph().topology(), factory);

            self.data.insert(expr.id(), presentation);
        }
    }

    pub(crate) fn get_mut(
        &mut self,
        niid: SoundExpressionId,
    ) -> Option<&mut ExpressionPresentation> {
        self.data.get_mut(&niid)
    }
}

pub struct AnyExpressionNodeObjectUiData {
    _id: ExpressionNodeId,
    state: RefCell<Box<dyn AnyObjectUiState>>,
    layout: ExpressionNodeLayout,
}

impl AnyExpressionNodeObjectUiData {
    pub(crate) fn layout(&self) -> ExpressionNodeLayout {
        self.layout
    }
}

impl ObjectUiData for AnyExpressionNodeObjectUiData {
    type GraphUi = ExpressionGraphUi;

    type RequiredData = ExpressionNodeLayout;

    fn new<S: ObjectUiState>(id: ExpressionNodeId, state: S, data: Self::RequiredData) -> Self {
        AnyExpressionNodeObjectUiData {
            _id: id,
            state: RefCell::new(Box::new(state)),
            layout: data,
        }
    }

    type ConcreteType<'a, T: ObjectUiState> = ExpressionNodeObjectUiData<'a, T>;

    fn downcast_with<
        T: ObjectUiState,
        F: FnOnce(
            ExpressionNodeObjectUiData<'_, T>,
            &mut ExpressionGraphUiState,
            &ExpressionGraphUiContext,
        ),
    >(
        &self,
        ui_state: &mut ExpressionGraphUiState,
        ctx: &ExpressionGraphUiContext<'_>,
        f: F,
    ) {
        let mut state_mut = self.state.borrow_mut();
        #[cfg(debug_assertions)]
        {
            let actual_name = state_mut.get_language_type_name();
            let state_any = state_mut.as_mut_any();
            debug_assert!(
                state_any.is::<T>(),
                "AnyExpressionNodeObjectUiData expected to receive state type {}, but got {:?} instead",
                type_name::<T>(),
                actual_name
            );
        }
        let state_any = state_mut.as_mut_any();
        let state = state_any.downcast_mut::<T>().unwrap();

        f(ExpressionNodeObjectUiData { state }, ui_state, ctx);
    }
}

pub struct ExpressionNodeObjectUiData<'a, T: ObjectUiState> {
    pub state: &'a mut T,
    // TODO: other presentation state, e.g. color?
}

pub struct ExpressionNodeObjectUiStates {
    data: HashMap<ExpressionNodeId, Rc<AnyExpressionNodeObjectUiData>>,
}

impl ExpressionNodeObjectUiStates {
    pub(super) fn new() -> ExpressionNodeObjectUiStates {
        ExpressionNodeObjectUiStates {
            data: HashMap::new(),
        }
    }

    pub(super) fn generate(
        topo: &ExpressionGraphTopology,
        factory: &UiFactory<ExpressionGraphUi>,
    ) -> ExpressionNodeObjectUiStates {
        let mut states = Self::new();

        for node in topo.nodes().values() {
            let object = node.instance_arc().as_graph_object();
            let state = factory.create_default_state(&object);
            states.set_object_data(node.id(), state);
        }

        states
    }

    pub(super) fn set_object_data(
        &mut self,
        id: ExpressionNodeId,
        state: AnyExpressionNodeObjectUiData,
    ) {
        self.data.insert(id, Rc::new(state));
    }

    pub(super) fn get_object_data(
        &self,
        id: ExpressionNodeId,
    ) -> Rc<AnyExpressionNodeObjectUiData> {
        Rc::clone(self.data.get(&id).unwrap())
    }

    pub(super) fn cleanup(&mut self, topology: &ExpressionGraphTopology) {
        self.data.retain(|id, _| topology.nodes().contains_key(id));
    }
}

pub struct ExpressionGraphUiState {
    object_states: ExpressionNodeObjectUiStates,
}

impl ExpressionGraphUiState {
    pub(crate) fn new(object_states: ExpressionNodeObjectUiStates) -> ExpressionGraphUiState {
        ExpressionGraphUiState { object_states }
    }
}

impl GraphUiState for ExpressionGraphUiState {
    type GraphUi = ExpressionGraphUi;

    fn get_object_ui_data(&self, id: ExpressionNodeId) -> Rc<AnyExpressionNodeObjectUiData> {
        self.object_states.get_object_data(id)
    }
}
