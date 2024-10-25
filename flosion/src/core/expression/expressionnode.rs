use std::{
    any::{type_name, Any},
    ops::Deref,
    rc::Rc,
};

use inkwell::values::{FloatValue, PointerValue};

use crate::{
    core::{
        jit::jit::Jit,
        objecttype::{ObjectType, WithObjectType},
        uniqueid::UniqueId,
    },
    ui_core::arguments::ParsedArguments,
};

use super::{
    expressiongraph::ExpressionGraph,
    expressionnodetools::ExpressionNodeTools,
    expressionobject::{AnyExpressionObjectHandle, ExpressionObject, ExpressionObjectHandle},
};

pub struct ExpressionNodeTag;

pub type ExpressionNodeId = UniqueId<ExpressionNodeTag>;

/// An ExpressionNode whose values are computed as a pure function of the inputs,
/// with no side effects or hidden state. Intended to be used for elementary
/// mathematical functions and easy, closed-form calculations.
pub trait PureExpressionNode: WithObjectType {
    fn new(tools: ExpressionNodeTools<'_>, args: &ParsedArguments) -> Result<Self, ()>
    where
        Self: Sized;

    // Generate instructions to compute a value from the given inputs
    fn compile<'ctx>(&self, jit: &mut Jit<'ctx>, inputs: &[FloatValue<'ctx>]) -> FloatValue<'ctx>;
}

/// A trait representing any type of expression node, both
/// pure and stateful. Intended mainly for trait objects
/// and easy grouping of the different types.
pub trait ExpressionNode {
    fn num_variables(&self) -> usize;

    fn compile<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
        inputs: &[FloatValue<'ctx>],
        state_ptrs: &[PointerValue<'ctx>],
    ) -> FloatValue<'ctx>;

    fn as_graph_object(self: Rc<Self>) -> AnyExpressionObjectHandle;
}

pub struct PureExpressionNodeWithId<T: PureExpressionNode> {
    instance: T,
    id: ExpressionNodeId,
}

impl<T: PureExpressionNode> PureExpressionNodeWithId<T> {
    pub(crate) fn new(instance: T, id: ExpressionNodeId) -> PureExpressionNodeWithId<T> {
        PureExpressionNodeWithId { instance, id }
    }

    pub(crate) fn id(&self) -> ExpressionNodeId {
        self.id
    }
}

impl<T: PureExpressionNode> Deref for PureExpressionNodeWithId<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.instance
    }
}

impl<T: 'static + PureExpressionNode> ExpressionNode for PureExpressionNodeWithId<T> {
    fn num_variables(&self) -> usize {
        0
    }

    fn compile<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
        inputs: &[FloatValue<'ctx>],
        variables: &[PointerValue<'ctx>],
    ) -> FloatValue<'ctx> {
        debug_assert_eq!(variables.len(), 0);
        jit.builder()
            .position_before(&jit.instruction_locations.end_of_loop);
        self.instance.compile(jit, inputs)
    }

    fn as_graph_object(self: Rc<Self>) -> AnyExpressionObjectHandle {
        AnyExpressionObjectHandle::new(self)
    }
}

impl<T: 'static + PureExpressionNode> ExpressionObject for PureExpressionNodeWithId<T> {
    fn create(
        graph: &mut ExpressionGraph,
        args: &ParsedArguments,
    ) -> Result<AnyExpressionObjectHandle, ()> {
        graph
            .add_pure_expression_node::<T>(args)
            .map(|h| h.into_graph_object())
            .map_err(|_| ()) // TODO: report error
    }

    fn get_type() -> ObjectType {
        T::TYPE
    }

    fn get_dynamic_type(&self) -> ObjectType {
        T::TYPE
    }

    fn get_id(&self) -> ExpressionNodeId {
        self.id
    }

    fn into_rc_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<Self>()
    }
}

pub struct PureExpressionNodeHandle<T: PureExpressionNode> {
    instance: Rc<PureExpressionNodeWithId<T>>,
}

// NOTE: Deriving Clone explicitly because #[derive(Clone)] stupidly
// requires T: Clone even if it isn't stored as a direct field
impl<T: PureExpressionNode> Clone for PureExpressionNodeHandle<T> {
    fn clone(&self) -> Self {
        Self {
            instance: Rc::clone(&self.instance),
        }
    }
}

impl<T: 'static + PureExpressionNode> PureExpressionNodeHandle<T> {
    pub(super) fn new(instance: Rc<PureExpressionNodeWithId<T>>) -> Self {
        Self { instance }
    }

    pub(super) fn from_graph_object(handle: AnyExpressionObjectHandle) -> Option<Self> {
        let rc_any = handle.into_instance_rc().into_rc_any();
        match rc_any.downcast::<PureExpressionNodeWithId<T>>() {
            Ok(obj) => Some(PureExpressionNodeHandle::new(obj)),
            Err(_) => None,
        }
    }

    pub fn id(&self) -> ExpressionNodeId {
        self.instance.id()
    }

    pub fn into_graph_object(self) -> AnyExpressionObjectHandle {
        AnyExpressionObjectHandle::new(self.instance)
    }
}

impl<T: PureExpressionNode> Deref for PureExpressionNodeHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.instance
    }
}

impl<T: 'static + PureExpressionNode> ExpressionObjectHandle for PureExpressionNodeHandle<T> {
    type ObjectType = PureExpressionNodeWithId<T>;

    fn from_graph_object(object: AnyExpressionObjectHandle) -> Option<Self> {
        PureExpressionNodeHandle::from_graph_object(object)
    }

    fn object_type() -> ObjectType {
        T::TYPE
    }
}

/// An Expression which might have hidden state and/or might require
/// special build-up and tear-down to be used. This includes calculations
/// involving reccurences, e.g. relying on previous results, as well
/// as data structures that e.g. require locking in order to read safely.
pub trait StatefulExpressionNode: WithObjectType {
    fn new(tools: ExpressionNodeTools<'_>, args: &ParsedArguments) -> Result<Self, ()>
    where
        Self: Sized;

    // The number of additional floating point variables that are
    // associated with each compiled instance of the node.
    // This is allowed to be zero.
    const NUM_VARIABLES: usize;

    // A type intended to be used to store instruction values and temporary
    // variables that are to be shared between the pre-loop, loop, and post-loop
    // phases of the node's evaluation. For example, a pointer value
    // pointing to data might be fetched and locked in the pre-loop phase,
    // dereferenced in the loop phase, and unlocked in the post-loop phase.
    type CompileState<'ctx>;

    // Generate instructions to produce the initial values of state variables.
    // This will be run the first time the compiled called when starting over.
    // The returned vector must have length Self::NUM_VARIABLES
    fn compile_start_over<'ctx>(&self, jit: &mut Jit<'ctx>) -> Vec<FloatValue<'ctx>>;

    // Generate instructions to perform any necessary work prior
    // to the main body of the compiled function, such as synchronization
    // (ideally non-blocking) or doing monitoring and statistics
    fn compile_pre_loop<'ctx>(&self, jit: &mut Jit<'ctx>) -> Self::CompileState<'ctx>;

    // Generate instructions to perform any necessary work after
    // the main body of the compiled function, such as synchronization
    // (ideally non-blocking) or doing monitoring and statistics
    fn compile_post_loop<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
        compile_state: &Self::CompileState<'ctx>,
    );

    // Generate instructions to compute a value from the given inputs

    // Generate instructions to read and update state variables and produce
    // each new value from the state variables and input values
    fn compile_loop<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
        inputs: &[FloatValue<'ctx>],
        variables: &[PointerValue<'ctx>],
        compile_state: &Self::CompileState<'ctx>,
    ) -> FloatValue<'ctx>;
}

pub struct StatefulExpressionNodeWithId<T: StatefulExpressionNode> {
    instance: T,
    id: ExpressionNodeId,
}

impl<T: StatefulExpressionNode> StatefulExpressionNodeWithId<T> {
    pub(crate) fn new(instance: T, id: ExpressionNodeId) -> StatefulExpressionNodeWithId<T> {
        StatefulExpressionNodeWithId { instance, id }
    }

    pub(crate) fn id(&self) -> ExpressionNodeId {
        self.id
    }
}

impl<T: StatefulExpressionNode> Deref for StatefulExpressionNodeWithId<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.instance
    }
}

impl<T: 'static + StatefulExpressionNode> ExpressionNode for StatefulExpressionNodeWithId<T> {
    fn num_variables(&self) -> usize {
        T::NUM_VARIABLES
    }

    fn compile<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
        inputs: &[FloatValue<'ctx>],
        state_ptrs: &[PointerValue<'ctx>],
    ) -> FloatValue<'ctx> {
        // Allocate stack variables for state variables
        jit.builder()
            .position_before(&jit.instruction_locations.end_of_entry);
        let stack_variables: Vec<PointerValue<'ctx>> = (0..self.num_variables())
            .map(|i| {
                jit.builder()
                    .build_alloca(
                        jit.types.f32_type,
                        &format!("node{}_state{}", self.id().value(), i),
                    )
                    .unwrap()
            })
            .collect();

        // ===========================================================
        // =       First-time initialization and starting over       =
        // ===========================================================
        // assign initial values to stack variables
        jit.builder()
            .position_before(&jit.instruction_locations.end_of_startover);
        let init_variable_values = self.compile_start_over(jit);
        debug_assert_eq!(init_variable_values.len(), self.num_variables());
        for (stack_var, init_value) in stack_variables.iter().zip(init_variable_values) {
            jit.builder().build_store(*stack_var, init_value).unwrap();
        }

        // ===========================================================
        // =                Non-first-time resumption                =
        // ===========================================================
        // copy state array values into stack variables
        jit.builder()
            .position_before(&jit.instruction_locations.end_of_resume);
        for (stack_var, ptr_state) in stack_variables.iter().zip(state_ptrs) {
            // tmp = *ptr_state
            let tmp = jit
                .builder()
                .build_load(jit.types.f32_type, *ptr_state, "tmp")
                .unwrap();
            // *stack_var = tmp
            jit.builder().build_store(*stack_var, tmp).unwrap();
        }

        // ===========================================================
        // =           Pre-loop resumption and preparation           =
        // ===========================================================
        // any custom pre-loop work
        jit.builder()
            .position_before(&jit.instruction_locations.end_of_pre_loop);
        let compile_state = self.instance.compile_pre_loop(jit);

        // ===========================================================
        // =            Post-loop persisting and tear-down           =
        // ===========================================================
        // at end of loop, copy stack variables into state array
        jit.builder()
            .position_before(&jit.instruction_locations.end_of_post_loop);
        for (stack_var, ptr_state) in stack_variables.iter().zip(state_ptrs) {
            // tmp = *stack_var
            let tmp = jit
                .builder()
                .build_load(jit.types.f32_type, *stack_var, "tmp")
                .unwrap();
            // *ptr_state = tmp
            jit.builder().build_store(*ptr_state, tmp).unwrap();
        }
        // any custom post-loop work
        self.instance.compile_post_loop(jit, &compile_state);

        // ===========================================================
        // =                        The loop                         =
        // ===========================================================
        jit.builder()
            .position_before(&jit.instruction_locations.end_of_loop);
        let loop_value = self.compile_loop(jit, inputs, &stack_variables, &compile_state);

        loop_value
    }

    fn as_graph_object(self: Rc<Self>) -> AnyExpressionObjectHandle {
        AnyExpressionObjectHandle::new(self)
    }
}

impl<T: 'static + StatefulExpressionNode> ExpressionObject for StatefulExpressionNodeWithId<T> {
    fn create(
        graph: &mut ExpressionGraph,
        args: &ParsedArguments,
    ) -> Result<AnyExpressionObjectHandle, ()> {
        graph
            .add_stateful_expression_node::<T>(args)
            .map(|h| h.into_graph_object())
            .map_err(|_| ()) // TODO: report error
    }

    fn get_type() -> ObjectType {
        T::TYPE
    }

    fn get_dynamic_type(&self) -> ObjectType {
        T::TYPE
    }

    fn get_id(&self) -> ExpressionNodeId {
        self.id
    }

    fn into_rc_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<Self>()
    }
}

pub struct StatefulExpressionNodeHandle<T: StatefulExpressionNode> {
    instance: Rc<StatefulExpressionNodeWithId<T>>,
}

// NOTE: Deriving Clone explicitly because #[derive(Clone)] stupidly
// requires T: Clone even if it isn't stored as a direct field
impl<T: StatefulExpressionNode> Clone for StatefulExpressionNodeHandle<T> {
    fn clone(&self) -> Self {
        Self {
            instance: Rc::clone(&self.instance),
        }
    }
}

impl<T: 'static + StatefulExpressionNode> StatefulExpressionNodeHandle<T> {
    pub(super) fn new(instance: Rc<StatefulExpressionNodeWithId<T>>) -> Self {
        Self { instance }
    }

    pub(super) fn from_graph_object(handle: AnyExpressionObjectHandle) -> Option<Self> {
        let any = handle.into_instance_rc().into_rc_any();
        match any.downcast::<StatefulExpressionNodeWithId<T>>() {
            Ok(obj) => Some(StatefulExpressionNodeHandle::new(obj)),
            Err(_) => None,
        }
    }

    pub fn id(&self) -> ExpressionNodeId {
        self.instance.id()
    }

    pub fn into_graph_object(self) -> AnyExpressionObjectHandle {
        AnyExpressionObjectHandle::new(self.instance)
    }
}
impl<T: StatefulExpressionNode> Deref for StatefulExpressionNodeHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.instance
    }
}

impl<T: 'static + StatefulExpressionNode> ExpressionObjectHandle
    for StatefulExpressionNodeHandle<T>
{
    type ObjectType = StatefulExpressionNodeWithId<T>;

    fn from_graph_object(object: AnyExpressionObjectHandle) -> Option<Self> {
        StatefulExpressionNodeHandle::from_graph_object(object)
    }

    fn object_type() -> ObjectType {
        T::TYPE
    }
}
