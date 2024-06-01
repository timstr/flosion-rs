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
    graph_ui::{ObjectUiData, ObjectUiState},
    lexicallayout::lexicallayout::ExpressionNodeLayout,
    object_ui_states::AnyObjectUiState,
    soundobjectuistate::SoundObjectUiStates,
};

pub struct ExpressionGraphUiState {
    // TODO: what is this for???
}

impl ExpressionGraphUiState {
    pub(super) fn new() -> ExpressionGraphUiState {
        ExpressionGraphUiState {}
    }

    pub(super) fn cleanup(&mut self) {}
}

pub(super) struct ExpressionUiCollection {
    data: HashMap<SoundExpressionId, (ExpressionGraphUiState, ExpressionPresentation)>,
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
        ui_state: ExpressionGraphUiState,
        presentation: ExpressionPresentation,
    ) {
        self.data.insert(niid, (ui_state, presentation));
    }

    pub(super) fn cleanup(
        &mut self,
        topology: &SoundGraphTopology,
        object_ui_states: &SoundObjectUiStates,
    ) {
        self.data
            .retain(|id, _| topology.expressions().contains_key(id));

        for (niid, (ui_state, presentation)) in &mut self.data {
            let number_topo = topology
                .expression(*niid)
                .unwrap()
                .expression_graph()
                .topology();
            ui_state.cleanup();
            presentation.cleanup(
                number_topo,
                &object_ui_states.expression_graph_object_state(*niid),
            );
        }
    }

    pub(crate) fn get_mut(
        &mut self,
        niid: SoundExpressionId,
    ) -> Option<(&mut ExpressionGraphUiState, &mut ExpressionPresentation)> {
        self.data.get_mut(&niid).map(|(a, b)| (a, b))
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
            &mut ExpressionGraphUiContext,
        ),
    >(
        &self,
        ui_state: &mut ExpressionGraphUiState,
        ctx: &mut ExpressionGraphUiContext<'_>,
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
