use std::sync::Arc;

use super::{
    context::Context, numberinput::NumberInputId, numbersource::NumberSource, numeric,
    soundgraphtopology::SoundGraphTopology,
};

// TODO: first version: just duplicate the number sources in the topoloy
// worry about compilation later (and about allocating scratch space and
// about varying buffer sizes and scalars)

// NOTE: some dynamic dispatch overhead per operation is okay for now,
// since anything else would be actual compilation.

// TODO: also worry about overloaded number inputs later. One target per
// input for now

// TODO: second version use inkwell for LLVM bindings and real jit compilation

pub(super) struct CompiledNumberInput {
    target: Option<Arc<dyn NumberSource>>,
    default_value: f32,
}

impl CompiledNumberInput {
    pub(super) fn compile(
        number_input_id: NumberInputId,
        topology: &SoundGraphTopology,
    ) -> CompiledNumberInput {
        let input_data = topology.number_input(number_input_id).unwrap();
        let target = match input_data.target() {
            Some(nsid) => Some(topology.number_source(nsid).unwrap().instance_arc()),
            None => None,
        };
        let default_value = input_data.default_value();
        CompiledNumberInput {
            target,
            default_value,
        }
    }

    pub(super) fn eval(&self, dst: &mut [f32], context: &Context) {
        match &self.target {
            Some(t) => t.eval(dst, context),
            None => numeric::fill(dst, self.default_value),
        }
    }
}
