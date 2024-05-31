use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[cfg(debug_assertions)]
use std::any::type_name;

use crate::core::{
    expression::{
        expressiongraphtopology::ExpressionGraphTopology, expressionnode::ExpressionNodeId,
    },
    sound::{soundgraphtopology::SoundGraphTopology, expression::SoundExpressionId},
};

use super::{
    graph_ui::{ObjectUiData, ObjectUiState},
    lexicallayout::lexicallayout::NumberSourceLayout,
    numbergraphui::NumberGraphUi,
    numbergraphuicontext::NumberGraphUiContext,
    object_ui_states::AnyObjectUiState,
    soundnumberinputui::SoundNumberInputPresentation,
    soundobjectuistate::SoundObjectUiStates,
};

pub struct NumberGraphUiState {
    // TODO: what is this for???
}

impl NumberGraphUiState {
    pub(super) fn new() -> NumberGraphUiState {
        NumberGraphUiState {}
    }

    pub(super) fn cleanup(&mut self) {}
}

pub(super) struct SoundNumberInputUiCollection {
    data: HashMap<SoundExpressionId, (NumberGraphUiState, SoundNumberInputPresentation)>,
}

impl SoundNumberInputUiCollection {
    pub(super) fn new() -> SoundNumberInputUiCollection {
        SoundNumberInputUiCollection {
            data: HashMap::new(),
        }
    }

    pub(super) fn set_ui_data(
        &mut self,
        niid: SoundExpressionId,
        ui_state: NumberGraphUiState,
        presentation: SoundNumberInputPresentation,
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
                &object_ui_states.number_graph_object_state(*niid),
            );
        }
    }

    pub(crate) fn get_mut(
        &mut self,
        niid: SoundExpressionId,
    ) -> Option<(&mut NumberGraphUiState, &mut SoundNumberInputPresentation)> {
        self.data.get_mut(&niid).map(|(a, b)| (a, b))
    }
}

pub struct AnyNumberObjectUiData {
    _id: ExpressionNodeId,
    state: RefCell<Box<dyn AnyObjectUiState>>,
    layout: NumberSourceLayout,
}

impl AnyNumberObjectUiData {
    pub(crate) fn layout(&self) -> NumberSourceLayout {
        self.layout
    }
}

impl ObjectUiData for AnyNumberObjectUiData {
    type GraphUi = NumberGraphUi;

    type RequiredData = NumberSourceLayout;

    fn new<S: ObjectUiState>(id: ExpressionNodeId, state: S, data: Self::RequiredData) -> Self {
        AnyNumberObjectUiData {
            _id: id,
            state: RefCell::new(Box::new(state)),
            layout: data,
        }
    }

    type ConcreteType<'a, T: ObjectUiState> = NumberObjectUiData<'a, T>;

    fn downcast_with<
        T: ObjectUiState,
        F: FnOnce(NumberObjectUiData<'_, T>, &mut NumberGraphUiState, &mut NumberGraphUiContext),
    >(
        &self,
        ui_state: &mut NumberGraphUiState,
        ctx: &mut NumberGraphUiContext<'_>,
        f: F,
    ) {
        let mut state_mut = self.state.borrow_mut();
        #[cfg(debug_assertions)]
        {
            let actual_name = state_mut.get_language_type_name();
            let state_any = state_mut.as_mut_any();
            debug_assert!(
                state_any.is::<T>(),
                "AnyNumberObjectUiData expected to receive state type {}, but got {:?} instead",
                type_name::<T>(),
                actual_name
            );
        }
        let state_any = state_mut.as_mut_any();
        let state = state_any.downcast_mut::<T>().unwrap();

        f(NumberObjectUiData { state }, ui_state, ctx);
    }
}

pub struct NumberObjectUiData<'a, T: ObjectUiState> {
    pub state: &'a mut T,
    // TODO: other presentation state, e.g. color?
}

pub struct NumberObjectUiStates {
    data: HashMap<ExpressionNodeId, Rc<AnyNumberObjectUiData>>,
}

impl NumberObjectUiStates {
    pub(super) fn new() -> NumberObjectUiStates {
        NumberObjectUiStates {
            data: HashMap::new(),
        }
    }

    pub(super) fn set_object_data(&mut self, id: ExpressionNodeId, state: AnyNumberObjectUiData) {
        self.data.insert(id, Rc::new(state));
    }

    pub(super) fn get_object_data(&self, id: ExpressionNodeId) -> Rc<AnyNumberObjectUiData> {
        Rc::clone(self.data.get(&id).unwrap())
    }

    pub(super) fn cleanup(&mut self, topology: &ExpressionGraphTopology) {
        self.data.retain(|id, _| topology.nodes().contains_key(id));
    }
}
