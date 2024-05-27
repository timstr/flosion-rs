use std::{any::type_name, ops::Deref, sync::Arc};

use inkwell::values::{FloatValue, PointerValue};
use serialization::Serializer;

use crate::core::{
    graph::graphobject::{
        GraphObject, GraphObjectHandle, ObjectHandle, ObjectInitialization, ObjectType,
        WithObjectType,
    },
    jit::codegen::CodeGen,
    uniqueid::UniqueId,
};

use super::{numbergraph::NumberGraph, numbersourcetools::NumberSourceTools};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NumberSourceId(usize);

impl NumberSourceId {
    pub(crate) fn new(value: usize) -> NumberSourceId {
        NumberSourceId(value)
    }
}

impl Default for NumberSourceId {
    fn default() -> NumberSourceId {
        NumberSourceId(1)
    }
}

impl UniqueId for NumberSourceId {
    fn value(&self) -> usize {
        self.0
    }

    fn next(&self) -> NumberSourceId {
        NumberSourceId(self.0 + 1)
    }
}

// A NumberSource whose values are computed as a pure function of the inputs,
// with no side effects or hidden state. Intended to be used for elementary
// mathematical functions and easy, closed-form calculations.
pub trait PureNumberSource: 'static + Sync + Send + WithObjectType {
    fn new(tools: NumberSourceTools<'_>, init: ObjectInitialization) -> Result<Self, ()>
    where
        Self: Sized;

    // Generate instructions to compute a value from the given inputs
    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx>;

    fn serialize(&self, _serializer: Serializer) {}
}

// A trait representing any time of NumberSource, both
// pure and stateful. Intended mainly for trait objects
// and easy grouping of the different types.
pub trait NumberSource: 'static + Sync + Send {
    fn num_variables(&self) -> usize;

    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
        state_ptrs: &[PointerValue<'ctx>],
    ) -> FloatValue<'ctx>;

    fn as_graph_object(self: Arc<Self>) -> GraphObjectHandle<NumberGraph>;
}

pub struct PureNumberSourceWithId<T: PureNumberSource> {
    source: T,
    id: NumberSourceId,
}

impl<T: PureNumberSource> PureNumberSourceWithId<T> {
    pub(crate) fn new(source: T, id: NumberSourceId) -> PureNumberSourceWithId<T> {
        PureNumberSourceWithId { source, id }
    }

    pub(crate) fn id(&self) -> NumberSourceId {
        self.id
    }
}

impl<T: PureNumberSource> Deref for PureNumberSourceWithId<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.source
    }
}

impl<T: PureNumberSource> NumberSource for PureNumberSourceWithId<T> {
    fn num_variables(&self) -> usize {
        0
    }

    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
        variables: &[PointerValue<'ctx>],
    ) -> FloatValue<'ctx> {
        debug_assert_eq!(variables.len(), 0);
        codegen
            .builder()
            .position_before(&codegen.instruction_locations.end_of_loop);
        self.source.compile(codegen, inputs)
    }

    fn as_graph_object(self: Arc<Self>) -> GraphObjectHandle<NumberGraph> {
        GraphObjectHandle::new(self)
    }
}

impl<T: PureNumberSource> GraphObject<NumberGraph> for PureNumberSourceWithId<T> {
    fn create(
        graph: &mut NumberGraph,
        init: ObjectInitialization,
    ) -> Result<GraphObjectHandle<NumberGraph>, ()> {
        graph
            .add_pure_number_source::<T>(init)
            .map(|h| h.into_graph_object())
            .map_err(|_| ()) // TODO: report error
    }

    fn get_type() -> ObjectType {
        T::TYPE
    }

    fn get_dynamic_type(&self) -> ObjectType {
        T::TYPE
    }

    fn get_id(&self) -> NumberSourceId {
        self.id
    }

    fn into_arc_any(self: Arc<Self>) -> Arc<dyn std::any::Any + Send + Sync> {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn serialize(&self, serializer: Serializer) {
        (&*self as &T).serialize(serializer);
    }
}

pub struct PureNumberSourceHandle<T: PureNumberSource> {
    instance: Arc<PureNumberSourceWithId<T>>,
}

impl<T: PureNumberSource> PureNumberSourceHandle<T> {
    pub(super) fn new(instance: Arc<PureNumberSourceWithId<T>>) -> Self {
        Self { instance }
    }

    pub(super) fn from_graph_object(handle: GraphObjectHandle<NumberGraph>) -> Option<Self> {
        let arc_any = handle.into_instance_arc().into_arc_any();
        match arc_any.downcast::<PureNumberSourceWithId<T>>() {
            Ok(obj) => Some(PureNumberSourceHandle::new(obj)),
            Err(_) => None,
        }
    }

    pub fn id(&self) -> NumberSourceId {
        self.instance.id()
    }

    pub fn into_graph_object(self) -> GraphObjectHandle<NumberGraph> {
        GraphObjectHandle::new(self.instance)
    }
}

impl<T: PureNumberSource> Deref for PureNumberSourceHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.instance
    }
}

impl<T: PureNumberSource> Clone for PureNumberSourceHandle<T> {
    fn clone(&self) -> Self {
        Self {
            instance: Arc::clone(&self.instance),
        }
    }
}

impl<T: PureNumberSource> ObjectHandle<NumberGraph> for PureNumberSourceHandle<T> {
    type ObjectType = PureNumberSourceWithId<T>;

    fn from_graph_object(object: GraphObjectHandle<NumberGraph>) -> Option<Self> {
        PureNumberSourceHandle::from_graph_object(object)
    }

    fn object_type() -> ObjectType {
        T::TYPE
    }
}

// A NumberSource which might have hidden state and/or might require
// special build-up and tear-down to be used. This includes calculations
// involving reccurences, e.g. relying on previous results, as well
// as data structures that e.g. require locking in order to read safely.
pub trait StatefulNumberSource: 'static + Sync + Send + WithObjectType {
    fn new(tools: NumberSourceTools<'_>, init: ObjectInitialization) -> Result<Self, ()>
    where
        Self: Sized;

    // The number of additional floating point variables that are
    // associated with each compiled instance of the number source.
    // This is allowed to be zero.
    const NUM_VARIABLES: usize;

    // A type intended to be used to store instruction values and temporary
    // variables that are to be shared between the pre-loop, loop, and post-loop
    // phases of the number source's evaluation. For example, a pointer value
    // pointing to data might be fetched and locked in the pre-loop phase,
    // dereferenced in the loop phase, and unlocked in the post-loop phase.
    type CompileState<'ctx>;

    // Generate instructions to produce the initial values of state variables.
    // This will be run the first time the compiled called after a reset.
    // The returned vector must have length Self::NUM_VARIABLES
    fn compile_reset<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> Vec<FloatValue<'ctx>>;

    // Generate instructions to perform any necessary work prior
    // to the main body of the compiled function, such as synchronization
    // (ideally non-blocking) or doing monitoring and statistics
    fn compile_pre_loop<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> Self::CompileState<'ctx>;

    // Generate instructions to perform any necessary work after
    // the main body of the compiled function, such as synchronization
    // (ideally non-blocking) or doing monitoring and statistics
    fn compile_post_loop<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        compile_state: &Self::CompileState<'ctx>,
    );

    // Generate instructions to compute a value from the given inputs

    // Generate instructions to read and update state variables and produce
    // each new value from the state variables and input values
    fn compile_loop<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
        variables: &[PointerValue<'ctx>],
        compile_state: &Self::CompileState<'ctx>,
    ) -> FloatValue<'ctx>;

    fn serialize(&self, _serializer: Serializer) {}
}

pub struct StatefulNumberSourceWithId<T: StatefulNumberSource> {
    source: T,
    id: NumberSourceId,
}

impl<T: StatefulNumberSource> StatefulNumberSourceWithId<T> {
    pub(crate) fn new(source: T, id: NumberSourceId) -> StatefulNumberSourceWithId<T> {
        StatefulNumberSourceWithId { source, id }
    }

    pub(crate) fn id(&self) -> NumberSourceId {
        self.id
    }
}

impl<T: StatefulNumberSource> Deref for StatefulNumberSourceWithId<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.source
    }
}

impl<T: StatefulNumberSource> NumberSource for StatefulNumberSourceWithId<T> {
    fn num_variables(&self) -> usize {
        T::NUM_VARIABLES
    }

    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
        state_ptrs: &[PointerValue<'ctx>],
    ) -> FloatValue<'ctx> {
        // Allocate stack variables for state variables
        codegen
            .builder()
            .position_before(&codegen.instruction_locations.end_of_entry);
        let stack_variables: Vec<PointerValue<'ctx>> = (0..self.num_variables())
            .map(|i| {
                codegen
                    .builder()
                    .build_alloca(
                        codegen.types.f32_type,
                        &format!("numbersource{}_state{}", self.id().value(), i),
                    )
                    .unwrap()
            })
            .collect();

        // ===========================================================
        // =           First-time initialization and reset           =
        // ===========================================================
        // assign initial values to stack variables
        codegen
            .builder()
            .position_before(&codegen.instruction_locations.end_of_reset);
        let init_variable_values = self.compile_reset(codegen);
        debug_assert_eq!(init_variable_values.len(), self.num_variables());
        for (stack_var, init_value) in stack_variables.iter().zip(init_variable_values) {
            codegen
                .builder()
                .build_store(*stack_var, init_value)
                .unwrap();
        }

        // ===========================================================
        // =                Non-first-time resumption                =
        // ===========================================================
        // copy state array values into stack variables
        codegen
            .builder()
            .position_before(&codegen.instruction_locations.end_of_resume);
        for (stack_var, ptr_state) in stack_variables.iter().zip(state_ptrs) {
            // tmp = *ptr_state
            let tmp = codegen.builder().build_load(*ptr_state, "tmp").unwrap();
            // *stack_var = tmp
            codegen.builder().build_store(*stack_var, tmp).unwrap();
        }

        // ===========================================================
        // =           Pre-loop resumption and preparation           =
        // ===========================================================
        // any custom pre-loop work
        codegen
            .builder()
            .position_before(&codegen.instruction_locations.end_of_pre_loop);
        let compile_state = self.source.compile_pre_loop(codegen);

        // ===========================================================
        // =            Post-loop persisting and tear-down           =
        // ===========================================================
        // at end of loop, copy stack variables into state array
        codegen
            .builder()
            .position_before(&codegen.instruction_locations.end_of_post_loop);
        for (stack_var, ptr_state) in stack_variables.iter().zip(state_ptrs) {
            // tmp = *stack_var
            let tmp = codegen.builder().build_load(*stack_var, "tmp").unwrap();
            // *ptr_state = tmp
            codegen.builder().build_store(*ptr_state, tmp).unwrap();
        }
        // any custom post-loop work
        self.source.compile_post_loop(codegen, &compile_state);

        // ===========================================================
        // =                        The loop                         =
        // ===========================================================
        codegen
            .builder()
            .position_before(&codegen.instruction_locations.end_of_loop);
        let loop_value = self.compile_loop(codegen, inputs, &stack_variables, &compile_state);

        loop_value
    }

    fn as_graph_object(self: Arc<Self>) -> GraphObjectHandle<NumberGraph> {
        GraphObjectHandle::new(self)
    }
}

impl<T: StatefulNumberSource> GraphObject<NumberGraph> for StatefulNumberSourceWithId<T> {
    fn create(
        graph: &mut NumberGraph,
        init: ObjectInitialization,
    ) -> Result<GraphObjectHandle<NumberGraph>, ()> {
        graph
            .add_stateful_number_source::<T>(init)
            .map(|h| h.into_graph_object())
            .map_err(|_| ()) // TODO: report error
    }

    fn get_type() -> ObjectType {
        T::TYPE
    }

    fn get_dynamic_type(&self) -> ObjectType {
        T::TYPE
    }

    fn get_id(&self) -> NumberSourceId {
        self.id
    }

    fn into_arc_any(self: Arc<Self>) -> Arc<dyn std::any::Any + Send + Sync> {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn serialize(&self, serializer: Serializer) {
        (&*self as &T).serialize(serializer);
    }
}

pub struct StatefulNumberSourceHandle<T: StatefulNumberSource> {
    instance: Arc<StatefulNumberSourceWithId<T>>,
}

impl<T: StatefulNumberSource> StatefulNumberSourceHandle<T> {
    pub(super) fn new(instance: Arc<StatefulNumberSourceWithId<T>>) -> Self {
        Self { instance }
    }

    pub(super) fn from_graph_object(handle: GraphObjectHandle<NumberGraph>) -> Option<Self> {
        let arc_any = handle.into_instance_arc().into_arc_any();
        match arc_any.downcast::<StatefulNumberSourceWithId<T>>() {
            Ok(obj) => Some(StatefulNumberSourceHandle::new(obj)),
            Err(_) => None,
        }
    }

    pub fn id(&self) -> NumberSourceId {
        self.instance.id()
    }

    pub fn into_graph_object(self) -> GraphObjectHandle<NumberGraph> {
        GraphObjectHandle::new(self.instance)
    }
}
impl<T: StatefulNumberSource> Deref for StatefulNumberSourceHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.instance
    }
}

impl<T: StatefulNumberSource> Clone for StatefulNumberSourceHandle<T> {
    fn clone(&self) -> Self {
        Self {
            instance: Arc::clone(&self.instance),
        }
    }
}

impl<T: StatefulNumberSource> ObjectHandle<NumberGraph> for StatefulNumberSourceHandle<T> {
    type ObjectType = StatefulNumberSourceWithId<T>;

    fn from_graph_object(object: GraphObjectHandle<NumberGraph>) -> Option<Self> {
        StatefulNumberSourceHandle::from_graph_object(object)
    }

    fn object_type() -> ObjectType {
        T::TYPE
    }
}
