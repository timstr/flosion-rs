use crate::core::sound::soundgraphtopology::SoundGraphTopology;

pub struct NodeGen<'a, 'ctx> {
    topology: &'a SoundGraphTopology,
    inkwell_context: &'ctx inkwell::context::Context,
}

impl<'a, 'ctx> NodeGen<'a, 'ctx> {
    pub(crate) fn new(
        topology: &'a SoundGraphTopology,
        inkwell_context: &'ctx inkwell::context::Context,
    ) -> NodeGen<'a, 'ctx> {
        NodeGen {
            topology,
            inkwell_context,
        }
    }

    pub(crate) fn inkwell_context(&self) -> &'ctx inkwell::context::Context {
        self.inkwell_context
    }

    pub(crate) fn topology(&self) -> &SoundGraphTopology {
        self.topology
    }
}
