use std::{
    any::Any,
    ops::{Deref, DerefMut},
};

use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};
use inkwell::values::{FloatValue, PointerValue};

use crate::{
    core::{
        jit::jit::Jit,
        objecttype::{ObjectType, WithObjectType},
        stashing::StashingContext,
        uniqueid::UniqueId,
    },
    ui_core::arguments::ParsedArguments,
};

use super::{
    expressioninput::{ExpressionInput, ExpressionInputId, ExpressionInputLocation},
    expressionobject::ExpressionObject,
};

pub struct ExpressionNodeTag;

pub type ExpressionNodeId = UniqueId<ExpressionNodeTag>;

pub trait ExpressionNodeVisitor {
    fn input(&mut self, _input: &ExpressionInput) {}
}

pub trait ExpressionNodeVisitorMut {
    fn input(&mut self, _input: &mut ExpressionInput) {}
}

/// An ExpressionNode whose values are computed as a pure function of the inputs,
/// with no side effects or hidden state. Intended to be used for elementary
/// mathematical functions and easy, closed-form calculations.
pub trait PureExpressionNode: WithObjectType {
    fn new(args: &ParsedArguments) -> Self
    where
        Self: Sized;

    // Generate instructions to compute a value from the given inputs
    fn compile<'ctx>(&self, jit: &mut Jit<'ctx>, inputs: &[FloatValue<'ctx>]) -> FloatValue<'ctx>;

    fn visit(&self, visitor: &mut dyn ExpressionNodeVisitor);
    fn visit_mut(&mut self, visitor: &mut dyn ExpressionNodeVisitorMut);
}

/// A trait representing any type of expression node, both
/// pure and stateful. Intended mainly for trait objects
/// and easy grouping of the different types.
pub trait AnyExpressionNode {
    fn id(&self) -> ExpressionNodeId;

    fn num_variables(&self) -> usize;

    fn compile<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
        inputs: &[FloatValue<'ctx>],
        state_ptrs: &[PointerValue<'ctx>],
    ) -> FloatValue<'ctx>;

    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;

    fn as_graph_object(&self) -> &dyn ExpressionObject;
    fn as_graph_object_mut(&mut self) -> &mut dyn ExpressionObject;

    fn visit(&self, visitor: &mut dyn ExpressionNodeVisitor);
    fn visit_mut(&mut self, visitor: &mut dyn ExpressionNodeVisitorMut);

    fn stash(&self, stasher: &mut Stasher<StashingContext>);
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError>;
}

/// An Expression which might have hidden state and/or might require
/// special build-up and tear-down to be used. This includes calculations
/// involving reccurences, e.g. relying on previous results, as well
/// as data structures that e.g. require locking in order to read safely.
pub trait ExpressionNode {
    fn new(args: &ParsedArguments) -> Self
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

    // Generate instructions to read and update state variables and produce
    // each new value from the state variables and input values
    fn compile_loop<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
        inputs: &[FloatValue<'ctx>],
        variables: &[PointerValue<'ctx>],
        compile_state: &Self::CompileState<'ctx>,
    ) -> FloatValue<'ctx>;

    fn visit(&self, visitor: &mut dyn ExpressionNodeVisitor);
    fn visit_mut(&mut self, visitor: &mut dyn ExpressionNodeVisitorMut);
}

impl<T: PureExpressionNode> ExpressionNode for T {
    fn new(args: &ParsedArguments) -> Self
    where
        Self: Sized,
    {
        T::new(args)
    }

    const NUM_VARIABLES: usize = 0;

    type CompileState<'ctx> = ();

    fn compile_start_over<'ctx>(&self, _jit: &mut Jit<'ctx>) -> Vec<FloatValue<'ctx>> {
        Vec::new()
    }

    fn compile_pre_loop<'ctx>(&self, _jit: &mut Jit<'ctx>) -> Self::CompileState<'ctx> {
        ()
    }

    fn compile_post_loop<'ctx>(
        &self,
        _jit: &mut Jit<'ctx>,
        _compile_state: &Self::CompileState<'ctx>,
    ) {
    }

    fn compile_loop<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
        inputs: &[FloatValue<'ctx>],
        _variables: &[PointerValue<'ctx>],
        _compile_state: &Self::CompileState<'ctx>,
    ) -> FloatValue<'ctx> {
        T::compile(self, jit, inputs)
    }

    fn visit(&self, visitor: &mut dyn ExpressionNodeVisitor) {
        T::visit(self, visitor);
    }
    fn visit_mut(&mut self, visitor: &mut dyn ExpressionNodeVisitorMut) {
        T::visit_mut(self, visitor);
    }
}

pub struct ExpressionNodeWithId<T> {
    id: ExpressionNodeId,
    instance: T,
}

impl<T: ExpressionNode> ExpressionNodeWithId<T> {
    pub(crate) fn new_default() -> ExpressionNodeWithId<T> {
        Self::new_from_args(&ParsedArguments::new_empty())
    }

    pub(crate) fn new_from_args(args: &ParsedArguments) -> ExpressionNodeWithId<T> {
        ExpressionNodeWithId {
            id: ExpressionNodeId::new_unique(),
            instance: T::new(args),
        }
    }

    pub(crate) fn id(&self) -> ExpressionNodeId {
        self.id
    }
}

impl<T> Deref for ExpressionNodeWithId<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.instance
    }
}

impl<T> DerefMut for ExpressionNodeWithId<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.instance
    }
}

impl<T> AnyExpressionNode for ExpressionNodeWithId<T>
where
    T: 'static + ExpressionNode + WithObjectType + Stashable<StashingContext> + UnstashableInplace,
{
    fn id(&self) -> ExpressionNodeId {
        self.id
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

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
            .position_at_end(jit.instruction_locations.post_loop);
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
            .position_at_end(jit.instruction_locations.loop_body);
        let loop_value = self.compile_loop(jit, inputs, &stack_variables, &compile_state);

        loop_value
    }

    fn as_graph_object(&self) -> &dyn ExpressionObject {
        self
    }
    fn as_graph_object_mut(&mut self) -> &mut dyn ExpressionObject {
        self
    }

    fn visit(&self, visitor: &mut dyn ExpressionNodeVisitor) {
        T::visit(&self.instance, visitor);
    }
    fn visit_mut(&mut self, visitor: &mut dyn ExpressionNodeVisitorMut) {
        T::visit_mut(&mut self.instance, visitor);
    }

    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        // id
        stasher.u64(self.id.value() as _);

        // contents
        stasher.object_proxy(|stasher| self.instance.stash(stasher));
    }
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        // id
        let id = ExpressionNodeId::new(unstasher.u64_always()? as _);
        if unstasher.time_to_write() {
            self.id = id;
        }

        // contents
        unstasher.object_inplace(&mut self.instance)?;

        Ok(())
    }
}

impl<'a> dyn AnyExpressionNode + 'a {
    pub(crate) fn downcast<T: 'static + ExpressionNode>(&self) -> Option<&ExpressionNodeWithId<T>> {
        self.as_any().downcast_ref()
    }

    pub(crate) fn downcast_mut<T: 'static + ExpressionNode>(
        &mut self,
    ) -> Option<&mut ExpressionNodeWithId<T>> {
        self.as_mut_any().downcast_mut()
    }

    pub(crate) fn with_input<R, F: FnMut(&ExpressionInput) -> R>(
        &self,
        input_id: ExpressionInputId,
        f: F,
    ) -> Option<R> {
        struct Visitor<R2, F2> {
            input_id: ExpressionInputId,
            result: Option<R2>,
            f: F2,
        }
        impl<R2, F2: FnMut(&ExpressionInput) -> R2> ExpressionNodeVisitor for Visitor<R2, F2> {
            fn input(&mut self, input: &ExpressionInput) {
                if input.id() == self.input_id {
                    debug_assert!(self.result.is_none());
                    self.result = Some((self.f)(input));
                }
            }
        }
        let mut visitor = Visitor {
            input_id,
            result: None,
            f,
        };
        self.visit(&mut visitor);
        visitor.result
    }

    pub(crate) fn with_input_mut<R, F: FnMut(&mut ExpressionInput) -> R>(
        &mut self,
        input_id: ExpressionInputId,
        f: F,
    ) -> Option<R> {
        struct Visitor<R2, F2> {
            input_id: ExpressionInputId,
            result: Option<R2>,
            f: F2,
        }
        impl<R2, F2: FnMut(&mut ExpressionInput) -> R2> ExpressionNodeVisitorMut for Visitor<R2, F2> {
            fn input(&mut self, input: &mut ExpressionInput) {
                if input.id() == self.input_id {
                    debug_assert!(self.result.is_none());
                    self.result = Some((self.f)(input));
                }
            }
        }
        let mut visitor = Visitor {
            input_id,
            result: None,
            f,
        };

        self.visit_mut(&mut visitor);
        visitor.result
    }

    pub(crate) fn foreach_input<F: FnMut(&ExpressionInput, ExpressionInputLocation)>(&self, f: F) {
        struct Visitor<F2> {
            node_id: ExpressionNodeId,
            f: F2,
        }

        impl<F2: FnMut(&ExpressionInput, ExpressionInputLocation)> ExpressionNodeVisitor for Visitor<F2> {
            fn input(&mut self, input: &ExpressionInput) {
                (self.f)(
                    input,
                    ExpressionInputLocation::NodeInput(self.node_id, input.id()),
                )
            }
        }

        self.visit(&mut Visitor {
            node_id: self.id(),
            f,
        });
    }

    pub(crate) fn foreach_input_mut<F: FnMut(&mut ExpressionInput, ExpressionInputLocation)>(
        &mut self,
        f: F,
    ) {
        struct Visitor<F2> {
            node_id: ExpressionNodeId,
            f: F2,
        }

        impl<F2: FnMut(&mut ExpressionInput, ExpressionInputLocation)> ExpressionNodeVisitorMut
            for Visitor<F2>
        {
            fn input(&mut self, input: &mut ExpressionInput) {
                (self.f)(
                    input,
                    ExpressionInputLocation::NodeInput(self.node_id, input.id()),
                )
            }
        }

        self.visit_mut(&mut Visitor {
            node_id: self.id(),
            f,
        });
    }

    pub(crate) fn input_locations(&self) -> Vec<ExpressionInputLocation> {
        let mut locations = Vec::new();
        self.foreach_input(|_, l| locations.push(l));
        locations
    }
}

impl<T> ExpressionObject for ExpressionNodeWithId<T>
where
    T: 'static + ExpressionNode + WithObjectType + Stashable<StashingContext> + UnstashableInplace,
{
    fn create(args: &ParsedArguments) -> ExpressionNodeWithId<T> {
        ExpressionNodeWithId::new_from_args(args)
    }

    fn id(&self) -> ExpressionNodeId {
        self.id
    }

    fn get_type() -> ObjectType {
        T::TYPE
    }

    fn get_dynamic_type(&self) -> ObjectType {
        T::TYPE
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_expression_node(&self) -> Option<&dyn AnyExpressionNode> {
        Some(self)
    }
    fn into_boxed_expression_node(self: Box<Self>) -> Option<Box<dyn AnyExpressionNode>> {
        Some(self)
    }
}
