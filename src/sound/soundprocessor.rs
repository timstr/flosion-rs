use crate::sound::gridspan::GridSpan;
use crate::sound::soundgraph::Context;
use crate::sound::soundgraph::SoundProcessorTools;
use crate::sound::soundinput::SoundInputId;
use crate::sound::soundstate::SoundState;
use crate::sound::statetable::StateTable;
use crate::sound::statetable::StateTablePartition;
use crate::sound::uniqueid::UniqueId;
use std::cell::RefCell;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SoundProcessorId(usize);

impl Default for SoundProcessorId {
    fn default() -> SoundProcessorId {
        SoundProcessorId(1)
    }
}
impl UniqueId for SoundProcessorId {
    fn value(&self) -> usize {
        self.0
    }
    fn next(&self) -> SoundProcessorId {
        SoundProcessorId(self.0 + 1)
    }
}

pub trait DynamicSoundProcessor {
    type StateType: SoundState;
    fn new<'a>(tools: SoundProcessorTools) -> Self;
    fn process_audio(&self, state: &mut Self::StateType, context: &mut Context);
}

pub trait StaticSoundProcessor {
    type StateType: SoundState;
    fn new<'a>(tools: SoundProcessorTools) -> Self;
    fn process_audio(&self, state: &mut Self::StateType, context: &mut Context);
    fn produces_output(&self) -> bool;
}

pub trait SoundProcessorWrapper {
    fn id(&self) -> SoundProcessorId;

    // Process the next chunk of audio
    fn process_audio(&self, context: &mut Context);

    // Whether the sound processor is static, e.g. having only one state ever,
    // not allowed to be duplicated, and usually representing an external device
    // such as a speaker or microphone
    fn is_static(&self) -> bool;

    fn num_states(&self) -> usize;

    fn find_state_index(&self, dst_input: SoundInputId, dst_state_index: usize) -> usize;

    // Whether the sound processor produces output, or else just consumes its
    // input buffer for some other purpose
    fn produces_output(&self) -> bool;

    // Registers a new input to which the sound processor is connected.
    // Doesn't add any states. Use insert_dst_states to do so
    fn add_dst(&mut self, dst_input: SoundInputId);

    // Unregisters an existing input.
    // Panics if there are any states still allocated
    fn remove_dst(&mut self, dst_input: SoundInputId);

    // Add additional states for a connected SoundInput for upstream
    // states that it has just added
    // Returns the span of states to add to all inputs
    fn insert_dst_states(&mut self, dst_input: SoundInputId, span: GridSpan) -> GridSpan;

    // Remove a subset of states for a connected SoundInput for upstream
    // states that it has just removed
    // Returns the span of states to remove from all inputs
    fn erase_dst_states(&mut self, dst_input: SoundInputId, span: GridSpan) -> GridSpan;

    // Reset a range of states for a connected SoundInput
    // Returns the span of states to reset in all inputs
    fn reset_states(&self, dst_input: SoundInputId, span: GridSpan) -> GridSpan;
}

pub struct WrappedDynamicSoundProcessor<T: DynamicSoundProcessor> {
    instance: T,
    pub(in crate::sound) id: Option<SoundProcessorId>,
    state_table: StateTable<T::StateType>,
    state_partition: StateTablePartition,
}

impl<T: DynamicSoundProcessor> WrappedDynamicSoundProcessor<T> {
    pub fn new(instance: T) -> WrappedDynamicSoundProcessor<T> {
        let id = None;
        let state_table = StateTable::new();
        let state_partition = StateTablePartition::new();
        WrappedDynamicSoundProcessor {
            instance,
            id,
            state_table,
            state_partition,
        }
    }

    pub fn instance(&self) -> &T {
        &self.instance
    }

    pub fn instance_mut(&mut self) -> &mut T {
        &mut self.instance
    }

    pub fn id(&self) -> SoundProcessorId {
        self.id.unwrap()
    }
}

impl<T: DynamicSoundProcessor> SoundProcessorWrapper for WrappedDynamicSoundProcessor<T> {
    fn id(&self) -> SoundProcessorId {
        self.id.unwrap()
    }

    fn process_audio(&self, context: &mut Context) {
        let mut state = self.state_table.get_state_mut(context.state_index());
        self.instance.process_audio(&mut state, context);
    }

    fn is_static(&self) -> bool {
        false
    }

    fn num_states(&self) -> usize {
        assert_eq!(
            self.state_partition.total_size(),
            self.state_table.total_size()
        );
        self.state_table.total_size()
    }

    fn find_state_index(&self, dst_input: SoundInputId, dst_state_index: usize) -> usize {
        self.state_partition.get_index(dst_input, dst_state_index)
    }

    fn produces_output(&self) -> bool {
        true
    }

    fn add_dst(&mut self, dst_input: SoundInputId) {
        assert_eq!(
            self.state_partition.total_size(),
            self.state_table.total_size()
        );
        self.state_partition.add_dst(dst_input);
        assert_eq!(
            self.state_partition.total_size(),
            self.state_table.total_size()
        );
    }

    fn remove_dst(&mut self, dst_input: SoundInputId) {
        assert_eq!(
            self.state_partition.total_size(),
            self.state_table.total_size()
        );
        self.state_partition.remove_dst(dst_input);
        assert_eq!(
            self.state_partition.total_size(),
            self.state_table.total_size()
        );
    }

    fn insert_dst_states(&mut self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        let s = self.state_partition.add_dst_states(dst_input, span);
        self.state_table.insert_states(s);
        s
    }

    fn erase_dst_states(&mut self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        let s = self.state_partition.remove_dst_states(dst_input, span);
        self.state_table.erase_states(s);
        s
    }

    fn reset_states(&self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        let s = self.state_partition.get_span(dst_input, span);
        self.state_table.reset_states(s);
        s
    }
}

struct StaticInputStates {
    input_id: SoundInputId,
    num_states: usize,
}

pub struct WrappedStaticSoundProcessor<T: StaticSoundProcessor> {
    instance: T,
    pub(in crate::sound) id: Option<SoundProcessorId>,
    state: RefCell<T::StateType>,
    dst_inputs: Vec<StaticInputStates>,
}

impl<T: StaticSoundProcessor> WrappedStaticSoundProcessor<T> {
    pub fn new(instance: T) -> WrappedStaticSoundProcessor<T> {
        let id = None;
        let dst_inputs = Vec::new();
        let state = RefCell::new(T::StateType::default());
        WrappedStaticSoundProcessor {
            instance,
            id,
            state,
            dst_inputs,
        }
    }

    pub fn instance(&self) -> &T {
        &self.instance
    }

    pub fn instance_mut(&mut self) -> &mut T {
        &mut self.instance
    }

    pub fn id(&self) -> SoundProcessorId {
        self.id.unwrap()
    }
}

// A static sound processor allows any number of sound inputs to be connected, but all
// will receive copies of the same single audio stream, and all may have at most one
// state.
impl<T: StaticSoundProcessor> SoundProcessorWrapper for WrappedStaticSoundProcessor<T> {
    fn id(&self) -> SoundProcessorId {
        self.id.unwrap()
    }

    fn process_audio(&self, context: &mut Context) {
        self.instance
            .process_audio(&mut self.state.borrow_mut(), context);
    }

    fn is_static(&self) -> bool {
        true
    }

    fn num_states(&self) -> usize {
        1
    }

    fn find_state_index(&self, dst_input: SoundInputId, dst_state_index: usize) -> usize {
        assert!(
            match self.dst_inputs.iter().find(|is| is.input_id == dst_input) {
                Some(is) => is.num_states == 1,
                None => false,
            }
        );
        assert_eq!(dst_state_index, 0);
        0
    }

    fn produces_output(&self) -> bool {
        self.instance.produces_output()
    }

    fn add_dst(&mut self, dst_input: SoundInputId) {
        assert!(self.produces_output());
        assert!(self
            .dst_inputs
            .iter()
            .find(|is| is.input_id == dst_input)
            .is_none());
        self.dst_inputs.push(StaticInputStates {
            input_id: dst_input,
            num_states: 0,
        });
    }

    fn remove_dst(&mut self, dst_input: SoundInputId) {
        assert!(self.produces_output());
        assert_eq!(
            self.dst_inputs
                .iter()
                .filter(|is| is.input_id == dst_input)
                .count(),
            1
        );
        let i = self
            .dst_inputs
            .iter()
            .position(|is| is.input_id == dst_input)
            .unwrap();
        let states = self.dst_inputs.remove(i);
        assert_eq!(states.num_states, 0);
    }

    fn insert_dst_states(&mut self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        assert!(self.produces_output());
        assert_eq!(
            self.dst_inputs
                .iter()
                .filter(|is| is.input_id == dst_input)
                .count(),
            1
        );
        if !(span.start_index() == 0 && span.num_items() == 1) {
            panic!();
        }
        let i = self
            .dst_inputs
            .iter()
            .position(|is| is.input_id == dst_input)
            .unwrap();
        let si = &mut self.dst_inputs[i];
        if si.num_states == 1 {
            panic!();
        }
        si.num_states = 1;
        GridSpan::new_empty()
    }

    fn erase_dst_states(&mut self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        assert!(self.produces_output());
        assert_eq!(
            self.dst_inputs
                .iter()
                .filter(|is| is.input_id == dst_input)
                .count(),
            1
        );
        if !(span.start_index() == 0 && span.num_items() == 1) {
            panic!();
        }
        let i = self
            .dst_inputs
            .iter()
            .position(|is| is.input_id == dst_input)
            .unwrap();
        let si = &mut self.dst_inputs[i];
        if si.num_states == 0 {
            panic!();
        }
        si.num_states = 0;
        GridSpan::new_empty()
    }

    fn reset_states(&self, _dst_input: SoundInputId, _span: GridSpan) -> GridSpan {
        // no-op, static sound sources can't be reset
        GridSpan::new_empty()
    }
}
