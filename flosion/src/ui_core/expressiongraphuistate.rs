use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc};

use hashstash::{Order, Stashable, Stasher, UnstashError, Unstashable, Unstasher};

use crate::core::{
    expression::{expressiongraph::ExpressionGraph, expressionnode::ExpressionNodeId},
    sound::{
        expression::{ProcessorExpressionId, ProcessorExpressionLocation},
        soundgraph::SoundGraph,
        soundprocessor::SoundProcessorId,
    },
};

use super::{
    arguments::ParsedArguments, expressionobjectui::ExpressionObjectUiFactory,
    lexicallayout::lexicallayout::LexicalLayout, stashing::UiUnstashingContext,
};

/// Container for holding the ui states of all nodes in a single
/// expression graph ui.
pub struct ExpressionNodeObjectUiStates {
    // TODO: store ExpressionNodeLayout per node here too
    // where to get them from though?
    //
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
    /// expression graph with default-created ui states.
    pub(super) fn generate(
        graph: &ExpressionGraph,
        factory: &ExpressionObjectUiFactory,
    ) -> ExpressionNodeObjectUiStates {
        let mut states = Self::new();

        for node in graph.nodes().values() {
            let object = node.as_graph_object();
            let object_type = object.get_dynamic_type();
            let object_ui = factory.get(object_type);
            let state = object_ui
                .make_ui_state(object, ParsedArguments::new_empty())
                .unwrap();
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
    /// exist in the given graph.
    pub(super) fn cleanup(&mut self, graph: &ExpressionGraph) {
        self.data.retain(|id, _| graph.nodes().contains_key(id));
    }
}

impl Stashable for ExpressionNodeObjectUiStates {
    fn stash(&self, stasher: &mut Stasher) {
        // TODO: stash those UI states
        // this will probably be best done by making a
        // specific trait of ui state that supports
        // serialization through a trait object.
        // Also, the type name of the object the ui
        // state is for will need to be stashed
        // in order to recreate it using the factory
    }
}

impl Unstashable<UiUnstashingContext<'_>> for ExpressionNodeObjectUiStates {
    fn unstash(unstasher: &mut Unstasher<UiUnstashingContext<'_>>) -> Result<Self, UnstashError> {
        // TODO
        Ok(ExpressionNodeObjectUiStates::new())
    }
}

/// The complete ui state for a single expression graph, as needed for
/// displaying any expression graph node's ui.
pub struct ExpressionGraphUiState {
    object_states: ExpressionNodeObjectUiStates,
}

impl ExpressionGraphUiState {
    /// Automatically generate the ui state for the given expression graph.
    // All expression node objects will be assigned default ui state.
    pub(crate) fn generate(
        graph: &ExpressionGraph,
        factory: &ExpressionObjectUiFactory,
    ) -> ExpressionGraphUiState {
        let object_states = ExpressionNodeObjectUiStates::generate(graph, factory);

        ExpressionGraphUiState { object_states }
    }

    /// Get a reference to the object ui states
    pub(crate) fn object_states(&self) -> &ExpressionNodeObjectUiStates {
        &self.object_states
    }

    /// Get a mutable reference to the object ui states
    pub(crate) fn object_states_mut(&mut self) -> &mut ExpressionNodeObjectUiStates {
        &mut self.object_states
    }

    /// Remove any data associated with objects that no longer exist in
    /// the given graph.
    fn cleanup(&mut self, graph: &ExpressionGraph) {
        self.object_states.cleanup(graph);
    }
}

impl Stashable for ExpressionGraphUiState {
    fn stash(&self, stasher: &mut Stasher<()>) {
        self.object_states.stash(stasher);
    }
}

impl Unstashable<UiUnstashingContext<'_>> for ExpressionGraphUiState {
    fn unstash(unstasher: &mut Unstasher<UiUnstashingContext>) -> Result<Self, UnstashError> {
        Ok(ExpressionGraphUiState {
            object_states: ExpressionNodeObjectUiStates::unstash(unstasher)?,
        })
    }
}

/// A container for holding the ui states of multiple, separate expression graphs.
/// This exists because a single sound graph can contain multiple expressions, and
/// so the single top-level sound graph UI likewise can contain many separate
/// expression graph UIs.
pub(crate) struct ExpressionUiCollection {
    data: HashMap<ProcessorExpressionLocation, (ExpressionGraphUiState, LexicalLayout)>,
}

impl ExpressionUiCollection {
    /// Create a new, empty ui collection without any expressions.
    pub(super) fn new() -> ExpressionUiCollection {
        ExpressionUiCollection {
            data: HashMap::new(),
        }
    }

    /// Get a mutable reference to the ui state for the given expression,
    /// if any exists.
    pub(crate) fn get_mut(
        &mut self,
        eid: ProcessorExpressionLocation,
    ) -> Option<(&mut ExpressionGraphUiState, &mut LexicalLayout)> {
        self.data.get_mut(&eid).map(|(a, b)| (a, b))
    }

    /// Remove any data associated with expressions or their components
    /// that no longer exist in the given sound graph.
    pub(super) fn cleanup(&mut self, graph: &SoundGraph, factory: &ExpressionObjectUiFactory) {
        // Delete data for removed expressions
        self.data.retain(|id, _| {
            // TODO: check that expression exists also
            graph.contains(&id.processor())
        });

        // Clean up the internal ui data of individual expressions
        for (eid, (expr_ui_state, layout)) in &mut self.data {
            graph
                .sound_processor(eid.processor())
                .unwrap()
                .with_expression(eid.expression(), |expr| {
                    expr_ui_state.cleanup(expr.graph());
                    layout.cleanup(expr.graph())
                });
        }

        // Add data for newly-added expressions
        for proc_data in graph.sound_processors().values() {
            proc_data.foreach_expression(|expr, location| {
                if self.data.contains_key(&location) {
                    return;
                }

                let mut ui_state = ExpressionGraphUiState::generate(expr.graph(), factory);

                let layout =
                    LexicalLayout::generate(expr.graph(), ui_state.object_states_mut(), factory);

                self.data.insert(location, (ui_state, layout));
            });
        }
    }
}

impl Stashable for ExpressionUiCollection {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.array_of_proxy_objects(
            self.data.iter(),
            |(proc_expr_loc, (expr_ui_state, lexical_layout)), stasher| {
                stasher.u64(proc_expr_loc.processor().value() as _);
                stasher.u64(proc_expr_loc.expression().value() as _);

                stasher.object(expr_ui_state);
                stasher.object(lexical_layout);
            },
            Order::Unordered,
        );
    }
}

impl Unstashable<UiUnstashingContext<'_>> for ExpressionUiCollection {
    fn unstash(unstasher: &mut Unstasher<UiUnstashingContext>) -> Result<Self, UnstashError> {
        let mut data = HashMap::new();

        unstasher.array_of_proxy_objects(|unstasher| {
            let location = ProcessorExpressionLocation::new(
                SoundProcessorId::new(unstasher.u64()? as _),
                ProcessorExpressionId::new(unstasher.u64()? as _),
            );

            let new_ui_state: ExpressionGraphUiState = unstasher.object()?;
            let new_layout: LexicalLayout = unstasher.object_with_context(())?;

            data.insert(location, (new_ui_state, new_layout));

            Ok(())
        })?;

        Ok(ExpressionUiCollection { data })
    }
}
