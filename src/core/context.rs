use super::{
    numberinput::NumberInputId,
    numeric,
    samplefrequency::SAMPLE_FREQUENCY,
    scratcharena::{ScratchArena, ScratchSlice},
    soundchunk::CHUNK_SIZE,
    soundgraphtopology::SoundGraphTopology,
    soundinput::{InputTiming, SoundInputId},
    soundprocessor::SoundProcessorId,
    statetree::{AnyData, ProcessorState, ProcessorTiming, State},
};

pub struct ProcessorStackFrame<'a> {
    parent: &'a StackFrame<'a>,
    state: AnyData<'a, SoundProcessorId>,
    timing: &'a ProcessorTiming,
}

impl<'a> ProcessorStackFrame<'a> {
    pub fn state(&self) -> &AnyData<'a, SoundProcessorId> {
        &self.state
    }
}

pub struct InputStackFrame<'a> {
    parent: &'a StackFrame<'a>,
    key: AnyData<'a, SoundInputId>,
    state: AnyData<'a, SoundInputId>,
    timing: &'a InputTiming,
}

impl<'a> InputStackFrame<'a> {
    pub fn key(&self) -> &AnyData<'a, SoundInputId> {
        &self.key
    }

    pub fn state(&self) -> &AnyData<'a, SoundInputId> {
        &self.state
    }

    pub fn timing(&self) -> &'a InputTiming {
        self.timing
    }
}

enum StackFrame<'a> {
    Processor(ProcessorStackFrame<'a>),
    Input(InputStackFrame<'a>),
    Root,
}

// struct ParentStackFrame<'a> {
//     frame: &'a StackFrame<'a>,
//     processor_state: AnyData<'a, SoundProcessorId>,
//     input_key: AnyData<'a, SoundInputId>,
//     input_state: AnyData<'a, SoundInputId>,
//     input_timing: &'a InputTiming,
// }

// struct StackFrame<'a> {
//     parent: Option<ParentStackFrame<'a>>,
// }

impl<'a> StackFrame<'a> {
    fn find_processor_frame(&self, processor_id: SoundProcessorId) -> &ProcessorStackFrame {
        match self {
            StackFrame::Processor(p) => {
                if p.state.owner_id() == processor_id {
                    p
                } else {
                    p.parent.find_processor_frame(processor_id)
                }
            }
            StackFrame::Input(i) => i.parent.find_processor_frame(processor_id),
            StackFrame::Root => {
                panic!("Attempted to find a processor frame which is not in the context call stack")
            }
        }
    }

    fn find_input_frame(&self, input_id: SoundInputId) -> &InputStackFrame {
        match self {
            StackFrame::Processor(p) => p.parent.find_input_frame(input_id),
            StackFrame::Input(i) => {
                if i.state.owner_id() == input_id {
                    i
                } else {
                    i.parent.find_input_frame(input_id)
                }
            }
            StackFrame::Root => {
                panic!("Attempted to find an input frame which is not in the context call stack")
            }
        }
    }

    fn find_processor_sample_offset(&self, processor_id: SoundProcessorId) -> usize {
        match self {
            StackFrame::Processor(p) => {
                let s = p.timing.elapsed_chunks() * CHUNK_SIZE;
                if p.state.owner_id() == processor_id {
                    s
                } else {
                    s + p.parent.find_processor_sample_offset(processor_id)
                }
            }
            StackFrame::Input(i) => {
                i.timing.sample_offset() + i.parent.find_processor_sample_offset(processor_id)
            }
            StackFrame::Root => {
                panic!("Attempted to find a processor frame which is not in the context call stack")
            }
        }
    }

    fn find_input_sample_offset(&self, input_id: SoundInputId) -> usize {
        match self {
            StackFrame::Processor(p) => {
                p.timing.elapsed_chunks() * CHUNK_SIZE + p.parent.find_input_sample_offset(input_id)
            }
            StackFrame::Input(i) => {
                if i.state.owner_id() == input_id {
                    0
                } else {
                    i.timing().sample_offset() + i.parent.find_input_sample_offset(input_id)
                }
            }
            StackFrame::Root => {
                panic!("Attempted to find an input frame which is not in the context call stack")
            }
        }
    }
}

pub struct Context<'a> {
    target_processor_id: Option<SoundProcessorId>,
    topology: &'a SoundGraphTopology,
    scratch_space: &'a ScratchArena,
    stack: StackFrame<'a>,
}

impl<'a> Context<'a> {
    pub fn new(
        target_processor_id: SoundProcessorId,
        topology: &'a SoundGraphTopology,
        scratch_space: &'a ScratchArena,
    ) -> Context<'a> {
        Context {
            target_processor_id: Some(target_processor_id),
            topology,
            scratch_space,
            stack: StackFrame::Root,
        }
    }

    pub fn push_input(
        &'a self,
        target: Option<SoundProcessorId>,
        key: AnyData<'a, SoundInputId>,
        state: AnyData<'a, SoundInputId>,
        timing: &'a InputTiming,
    ) -> Context<'a> {
        Context {
            target_processor_id: target,
            stack: StackFrame::Input(InputStackFrame {
                parent: &self.stack,
                key,
                state,
                timing,
            }),
            topology: self.topology,
            scratch_space: self.scratch_space,
        }
    }

    pub fn push_processor_state<T: State>(&'a self, state: &'a ProcessorState<T>) -> Context<'a> {
        Context {
            target_processor_id: None,
            stack: StackFrame::Processor(ProcessorStackFrame {
                parent: &self.stack,
                state: AnyData::new(self.target_processor_id.unwrap(), state.state()),
                timing: state.timing(),
            }),
            topology: self.topology,
            scratch_space: self.scratch_space,
        }
    }

    pub fn find_processor_frame(&self, processor_id: SoundProcessorId) -> &ProcessorStackFrame {
        self.stack.find_processor_frame(processor_id)
    }

    pub fn evaluate_number_input(&self, input_id: NumberInputId, dst: &mut [f32]) {
        // TODO: avoid a second hashmap lookup here by storing Arc to number source
        // in each input
        // TODO (much later): consider avoiding all lookups by storing an optional
        // Arc to the number source in the input node directly, if/when a mechanism
        // exists for modifying state trees in place
        let input = self.topology.number_inputs().get(&input_id).unwrap();
        if input.target().is_none() {
            numeric::fill(dst, 0.0);
            return;
        }
        let source = self
            .topology
            .number_sources()
            .get(&input.target().unwrap())
            .unwrap();
        source.instance().eval(dst, self);
    }

    pub fn get_scratch_space(&self, size: usize) -> ScratchSlice {
        self.scratch_space.borrow_slice(size)
    }

    // TODO: wrap this return value in something nicer
    pub fn find_input_frame(&self, input_id: SoundInputId) -> &InputStackFrame {
        self.stack.find_input_frame(input_id)
    }

    fn current_time_impl(samples_at_start: usize, dst: &mut [f32]) {
        let seconds_per_sample = 1.0 / SAMPLE_FREQUENCY as f32;
        let t0 = samples_at_start as f32 * seconds_per_sample;
        let t1 = (samples_at_start + dst.len()) as f32 * seconds_per_sample;
        // TODO: check whether dst is temporal or not
        numeric::linspace(dst, t0, t1);
    }

    pub fn current_time_at_sound_processor(&self, processor_id: SoundProcessorId, dst: &mut [f32]) {
        let s = self.stack.find_processor_sample_offset(processor_id);
        Self::current_time_impl(s, dst);
    }

    pub fn current_time_at_sound_input(&self, input_id: SoundInputId, dst: &mut [f32]) {
        let s = self.stack.find_input_sample_offset(input_id);
        Self::current_time_impl(s, dst);
    }
}
