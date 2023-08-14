use std::{any::type_name, cell::RefCell, collections::HashMap};

use crate::core::number::{numbergraphtopology::NumberGraphTopology, numbersource::NumberSourceId};

use super::{
    graph_ui::ObjectUiData, lexicallayout::LexicalLayout, numbergraphui::NumberGraphUi,
    numbergraphuicontext::NumberGraphUiContext, object_ui::ObjectUiState,
    object_ui_states::AnyObjectUiState,
};

pub struct NumberGraphUiState {
    lexical_layout: LexicalLayout,
}

impl NumberGraphUiState {
    pub(super) fn new(topo: &NumberGraphTopology) -> NumberGraphUiState {
        NumberGraphUiState {
            lexical_layout: LexicalLayout::generate(topo),
        }
    }

    pub(super) fn cleanup(&mut self, topology: &NumberGraphTopology) {
        self.lexical_layout.cleanup(topology);
    }
}

pub struct AnyNumberObjectUiData {
    id: NumberSourceId,
    state: Box<dyn AnyObjectUiState>,
}

impl ObjectUiData for AnyNumberObjectUiData {
    type GraphUi = NumberGraphUi;

    type ConcreteType<'a, T: ObjectUiState> = NumberObjectUiData<'a, T>;

    fn downcast<'a, T: ObjectUiState>(
        &'a mut self,
        ui_state: &NumberGraphUiState,
        ctx: &NumberGraphUiContext<'_>,
    ) -> Self::ConcreteType<'a, T> {
        let state_mut = &mut *self.state;
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

        NumberObjectUiData { state }
    }
}

pub struct NumberObjectUiData<'a, T: ObjectUiState> {
    pub state: &'a mut T,
    // TODO: other presentation state, e.g. color?
}

struct ObjectData {
    state: RefCell<AnyNumberObjectUiData>,
    // TODO: other presentation state, e.g. color?
}

pub struct NumberObjectUiStates {
    data: HashMap<NumberSourceId, ObjectData>,
}

impl NumberObjectUiStates {
    pub(super) fn new() -> NumberObjectUiStates {
        NumberObjectUiStates {
            data: HashMap::new(),
        }
    }

    pub(super) fn set_object_data(&mut self, id: NumberSourceId, state: Box<dyn AnyObjectUiState>) {
        self.data.insert(
            id,
            ObjectData {
                state: RefCell::new(AnyNumberObjectUiData { id, state }),
            },
        );
    }

    pub(super) fn get_object_data(&self, id: NumberSourceId) -> &RefCell<AnyNumberObjectUiData> {
        &self.data.get(&id).unwrap().state
    }

    pub(super) fn cleanup(&mut self, topology: &NumberGraphTopology) {
        self.data
            .retain(|id, _| topology.number_sources().contains_key(id));
    }
}
