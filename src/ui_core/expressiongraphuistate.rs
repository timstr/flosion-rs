use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc};

use crate::core::{
    expression::{
        expressiongraphtopology::ExpressionGraphTopology, expressionnode::ExpressionNodeId,
    },
    sound::{expression::SoundExpressionId, soundgraphtopology::SoundGraphTopology},
};

use super::{
    expressiongraphui::ExpressionGraphUi, graph_ui::GraphUiState,
    lexicallayout::lexicallayout::LexicalLayout, ui_factory::UiFactory,
};

/// Container for holding the ui states of all nodes in a single
/// expression graph ui.
pub struct ExpressionNodeObjectUiStates {
    data: HashMap<ExpressionNodeId, Rc<RefCell<dyn Any>>>,
}

impl ExpressionNodeObjectUiStates {
    /// Create a new instance which doesn't contain any ui states for
    /// any objects
    pub(super) fn new() -> ExpressionNodeObjectUiStates {
        ExpressionNodeObjectUiStates {
            data: HashMap::new(),
        }
    }

    /// Automatically fill the ui states for all objects in the given
    /// expression graph topology with default-created ui states.
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

    /// Replace the ui state for a single object. The concrete
    /// type of the ui state must match that expected by the
    /// object's ui, otherwise there will be an error later
    /// when the object's ui attempts to cast the ui state.
    pub(super) fn set_object_data(&mut self, id: ExpressionNodeId, state: Rc<RefCell<dyn Any>>) {
        self.data.insert(id, state);
    }

    /// Retrieve the ui state for a single object.
    pub(super) fn get_object_data(&self, id: ExpressionNodeId) -> Rc<RefCell<dyn Any>> {
        Rc::clone(self.data.get(&id).unwrap())
    }

    /// Remove any state associated with objects that no longer
    /// exist in the given topology.
    pub(super) fn cleanup(&mut self, topology: &ExpressionGraphTopology) {
        self.data.retain(|id, _| topology.nodes().contains_key(id));
    }
}

/// The complete ui state for a single expression graph, as needed for
/// displaying any expression graph node's ui.
pub struct ExpressionGraphUiState {
    object_states: ExpressionNodeObjectUiStates,
}

impl ExpressionGraphUiState {
    /// Automatically generate the ui state for the given expression graph
    /// topology. All expression node objects will be assigned default ui state.
    pub(crate) fn generate(
        topo: &ExpressionGraphTopology,
        factory: &UiFactory<ExpressionGraphUi>,
    ) -> ExpressionGraphUiState {
        let object_states = ExpressionNodeObjectUiStates::generate(topo, factory);

        ExpressionGraphUiState { object_states }
    }

    /// Get a mutable reference to the object ui states
    pub(crate) fn object_states_mut(&mut self) -> &mut ExpressionNodeObjectUiStates {
        &mut self.object_states
    }

    /// Remove any data associated with objects that no longer exist in
    /// the given topology.
    fn cleanup(&mut self, topo: &ExpressionGraphTopology) {
        self.object_states.cleanup(topo);
    }
}

impl GraphUiState for ExpressionGraphUiState {
    type GraphUi = ExpressionGraphUi;

    fn get_object_ui_data(&self, id: ExpressionNodeId) -> Rc<RefCell<dyn Any>> {
        self.object_states.get_object_data(id)
    }
}

/// A container for holding the ui states of multiple, separate expression graphs.
/// This exists because a single sound graph can contain multiple expressions, and
/// so the single top-level sound graph UI likewise can contain many separate
/// expression graph UIs.
pub(crate) struct ExpressionUiCollection {
    data: HashMap<SoundExpressionId, (ExpressionGraphUiState, LexicalLayout)>,
}

impl ExpressionUiCollection {
    /// Create a new, empty ui collection without any expressions.
    pub(super) fn new() -> ExpressionUiCollection {
        ExpressionUiCollection {
            data: HashMap::new(),
        }
    }

    /// Replace the ui state for a single expression
    pub(super) fn set_ui_data(
        &mut self,
        eid: SoundExpressionId,
        ui_state: ExpressionGraphUiState,
        layout: LexicalLayout,
    ) {
        self.data.insert(eid, (ui_state, layout));
    }

    /// Get a mutable reference to the ui state for the given expression,
    /// if any exists.
    pub(crate) fn get_mut(
        &mut self,
        eid: SoundExpressionId,
    ) -> Option<(&mut ExpressionGraphUiState, &mut LexicalLayout)> {
        self.data.get_mut(&eid).map(|(a, b)| (a, b))
    }

    /// Remove any data associated with expressions or their components
    /// that no longer exist in the given sound graph topology.
    pub(super) fn cleanup(
        &mut self,
        topology: &SoundGraphTopology,
        factory: &UiFactory<ExpressionGraphUi>,
    ) {
        // Delete data for removed expressions
        self.data
            .retain(|id, _| topology.expressions().contains_key(id));

        // Clean up the internal ui data of individual expressions
        for (eid, (expr_ui_state, layout)) in &mut self.data {
            let expr_topo = topology
                .expression(*eid)
                .unwrap()
                .expression_graph()
                .topology();
            expr_ui_state.cleanup(expr_topo);
            layout.cleanup(expr_topo)
        }

        // Add data for newly-added expressions
        for expr in topology.expressions().values() {
            if self.data.contains_key(&expr.id()) {
                continue;
            }

            let mut ui_state =
                ExpressionGraphUiState::generate(expr.expression_graph().topology(), factory);

            let layout = LexicalLayout::generate(
                expr.expression_graph().topology(),
                ui_state.object_states_mut(),
            );

            self.data.insert(expr.id(), (ui_state, layout));
        }
    }
}
