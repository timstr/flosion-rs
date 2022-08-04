use super::{
    numberinput::NumberInputId,
    numeric,
    samplefrequency::SAMPLE_FREQUENCY,
    scratcharena::{ScratchArena, ScratchSlice},
    soundchunk::CHUNK_SIZE,
    soundgraphtopology::SoundGraphTopology,
    soundinput::{InputTiming, SoundInputId},
    soundprocessor::SoundProcessorId,
    statetree::AnyData,
};

struct ParentStackFrame<'a> {
    frame: &'a StackFrame<'a>,
    processor_state: AnyData<'a, SoundProcessorId>,
    input_key: AnyData<'a, SoundInputId>,
    input_state: AnyData<'a, SoundInputId>,
    input_timing: &'a InputTiming,
}

struct StackFrame<'a> {
    parent: Option<ParentStackFrame<'a>>,
}

impl<'a> StackFrame<'a> {
    fn find_processor_frame(&self, processor_id: SoundProcessorId) -> &ParentStackFrame {
        match &self.parent {
            Some(p) => {
                if p.processor_state.owner_id() == processor_id {
                    p
                } else {
                    p.frame.find_processor_frame(processor_id)
                }
            }
            None => {
                panic!("Attempted to find a processor frame which is not in the context call stack")
            }
        }
    }

    fn find_input_frame(&self, input_id: SoundInputId) -> &ParentStackFrame {
        match &self.parent {
            Some(p) => {
                if p.input_key.owner_id() == input_id {
                    p
                } else {
                    p.frame.find_input_frame(input_id)
                }
            }
            None => {
                panic!("Attempted to find an input frame which is not in the context call stack")
            }
        }
    }

    fn find_processor_sample_offset(&self, processor_id: SoundProcessorId) -> usize {
        match &self.parent {
            Some(p) => {
                let s = p.input_timing.total_samples();
                if p.processor_state.owner_id() == processor_id {
                    s
                } else {
                    s + p.frame.find_processor_sample_offset(processor_id)
                }
            }
            None => {
                panic!("Attempted to find a processor frame which is not in the context call stack")
            }
        }
    }

    fn find_input_sample_offset(&self, input_id: SoundInputId) -> usize {
        match &self.parent {
            Some(p) => {
                if p.input_state.owner_id() == input_id {
                    0
                } else {
                    let s = p.input_timing.total_samples();
                    s + p.frame.find_input_sample_offset(input_id)
                }
            }
            None => {
                panic!("Attempted to find an input frame which is not in the context call stack")
            }
        }
    }
}

pub struct Context<'a> {
    topology: &'a SoundGraphTopology,
    scratch_space: &'a ScratchArena,
    stack: StackFrame<'a>,
}

impl<'a> Context<'a> {
    pub fn new(topology: &'a SoundGraphTopology, scratch_space: &'a ScratchArena) -> Context<'a> {
        Context {
            topology,
            scratch_space,
            stack: StackFrame { parent: None },
        }
    }

    pub(super) fn push_input(
        &'a self,
        processor_state: AnyData<'a, SoundProcessorId>,
        input_key: AnyData<'a, SoundInputId>,
        input_state: AnyData<'a, SoundInputId>,
        input_timing: &'a InputTiming,
    ) -> Context<'a> {
        Context {
            stack: StackFrame {
                parent: Some(ParentStackFrame {
                    frame: &self.stack,
                    processor_state,
                    input_key,
                    input_state,
                    input_timing,
                }),
            },
            topology: self.topology,
            scratch_space: self.scratch_space,
        }
    }

    pub fn find_processor_state(
        &self,
        processor_id: SoundProcessorId,
    ) -> AnyData<SoundProcessorId> {
        self.stack
            .find_processor_frame(processor_id)
            .processor_state
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
    pub fn find_keyed_input_state(
        &self,
        input_id: super::soundinput::SoundInputId,
    ) -> (AnyData<SoundInputId>, AnyData<SoundInputId>, &InputTiming) {
        let frame = self.stack.find_input_frame(input_id);
        (frame.input_key, frame.input_state, frame.input_timing)
    }

    fn current_time_impl(samples_at_start: usize, dst: &mut [f32]) {
        let seconds_per_sample = SAMPLE_FREQUENCY as f32 / CHUNK_SIZE as f32;
        // TODO: check whether dst is temporal or not
        numeric::linspace(
            dst,
            samples_at_start as f32 * seconds_per_sample,
            (samples_at_start + dst.len()) as f32 * seconds_per_sample,
        );
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
