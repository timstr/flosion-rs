use std::{any::type_name, cell::RefCell, collections::HashMap};

use crate::core::{
    number::{numbergraphtopology::NumberGraphTopology, numbersource::NumberSourceId},
    sound::{soundgraphtopology::SoundGraphTopology, soundnumberinput::SoundNumberInputId},
};

use super::{
    graph_ui::{ObjectUiData, ObjectUiState},
    lexicallayout::lexicallayout::NumberSourceLayout,
    numbergraphui::NumberGraphUi,
    numbergraphuicontext::NumberGraphUiContext,
    object_ui_states::AnyObjectUiState,
    soundnumberinputui::SoundNumberInputPresentation,
};

pub struct NumberGraphUiState {
    // TODO: what is this for???
}

impl NumberGraphUiState {
    pub(super) fn new() -> NumberGraphUiState {
        NumberGraphUiState {}
    }

    pub(super) fn cleanup(&mut self, topology: &NumberGraphTopology) {}
}

pub(super) struct SoundNumberInputUiCollection {
    data: HashMap<SoundNumberInputId, (NumberGraphUiState, SoundNumberInputPresentation)>,
}

impl SoundNumberInputUiCollection {
    pub(super) fn new() -> SoundNumberInputUiCollection {
        SoundNumberInputUiCollection {
            data: HashMap::new(),
        }
    }

    pub(super) fn set_ui_data(
        &mut self,
        niid: SoundNumberInputId,
        ui_state: NumberGraphUiState,
        presentation: SoundNumberInputPresentation,
    ) {
        self.data.insert(niid, (ui_state, presentation));
    }

    pub(super) fn cleanup(&mut self, topology: &SoundGraphTopology) {
        self.data
            .retain(|id, _| topology.number_inputs().contains_key(id));

        for (niid, (ui_state, presentation)) in &mut self.data {
            let number_topo = topology
                .number_input(*niid)
                .unwrap()
                .number_graph()
                .topology();
            ui_state.cleanup(number_topo);
            presentation.cleanup(number_topo);
        }
    }

    pub(crate) fn get_mut(
        &mut self,
        niid: SoundNumberInputId,
    ) -> Option<(&mut NumberGraphUiState, &mut SoundNumberInputPresentation)> {
        self.data.get_mut(&niid).map(|(a, b)| (a, b))
    }
}

pub struct AnyNumberObjectUiData {
    id: NumberSourceId,
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

    fn new<S: ObjectUiState>(id: NumberSourceId, state: S, data: Self::RequiredData) -> Self {
        AnyNumberObjectUiData {
            id,
            state: RefCell::new(Box::new(state)),
            layout: data,
        }
    }

    type ConcreteType<'a, T: ObjectUiState> = NumberObjectUiData<'a, T>;

    fn downcast_with<
        T: ObjectUiState,
        F: FnOnce(NumberObjectUiData<'_, T>, &mut NumberGraphUiState),
    >(
        &self,
        ui_state: &mut NumberGraphUiState,
        ctx: &NumberGraphUiContext<'_>,
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

        f(NumberObjectUiData { state }, ui_state);
    }
}

pub struct NumberObjectUiData<'a, T: ObjectUiState> {
    pub state: &'a mut T,
    // TODO: other presentation state, e.g. color?
}

pub struct NumberObjectUiStates {
    data: HashMap<NumberSourceId, AnyNumberObjectUiData>,
}

impl NumberObjectUiStates {
    pub(super) fn new() -> NumberObjectUiStates {
        NumberObjectUiStates {
            data: HashMap::new(),
        }
    }

    pub(super) fn set_object_data(&mut self, id: NumberSourceId, state: AnyNumberObjectUiData) {
        self.data.insert(id, state);
    }

    pub(super) fn get_object_data(&self, id: NumberSourceId) -> &AnyNumberObjectUiData {
        self.data.get(&id).unwrap()
    }

    pub(super) fn cleanup(&mut self, topology: &NumberGraphTopology) {
        self.data
            .retain(|id, _| topology.number_sources().contains_key(id));
    }
}
