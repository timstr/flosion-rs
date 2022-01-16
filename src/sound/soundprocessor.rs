use parking_lot::RwLock;

use crate::sound::gridspan::GridSpan;
use crate::sound::soundinput::SoundInputId;
use crate::sound::soundstate::SoundState;
use crate::sound::statetable::StateTable;
use crate::sound::statetable::StateTablePartition;
use crate::sound::uniqueid::UniqueId;

use std::sync::Arc;

use super::context::Context;
use super::context::ProcessorContext;
use super::soundchunk::SoundChunk;
use super::soundprocessortools::SoundProcessorTools;
use super::statetable::StateTableLock;

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

pub trait DynamicSoundProcessor: Sync + Send {
    type StateType: SoundState;
    fn new(
        tools: &mut SoundProcessorTools<'_>,
        data: &Arc<DynamicSoundProcessorData<Self::StateType>>,
    ) -> Self;
    fn process_audio(&self, context: ProcessorContext<'_, Self::StateType>);
}

pub trait StaticSoundProcessor: Sync + Send {
    type StateType: SoundState;
    fn new(
        tools: &mut SoundProcessorTools<'_>,
        data: &Arc<StaticSoundProcessorData<Self::StateType>>,
    ) -> Self;
    fn process_audio(&self, context: ProcessorContext<'_, Self::StateType>);
    fn produces_output(&self) -> bool;
    fn on_start_processing(&self) {}
    fn on_stop_processing(&self) {}
}

pub trait SoundProcessorWrapper: Sync + Send {
    fn id(&self) -> SoundProcessorId;

    // Process the next chunk of audio
    fn process_audio(&self, output_buffer: &mut SoundChunk, context: Context);

    fn on_start_processing(&self);

    fn on_stop_processing(&self);

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
    fn add_dst(&self, dst_input: SoundInputId);

    // Unregisters an existing input.
    // Panics if there are any states still allocated
    fn remove_dst(&self, dst_input: SoundInputId);

    // Add additional states for a connected SoundInput for upstream
    // states that it has just added
    // Returns the span of states to add to all inputs
    fn insert_dst_states(&self, dst_input: SoundInputId, span: GridSpan) -> GridSpan;

    // Remove a subset of states for a connected SoundInput for upstream
    // states that it has just removed
    // Returns the span of states to remove from all inputs
    fn erase_dst_states(&self, dst_input: SoundInputId, span: GridSpan) -> GridSpan;

    // Reset a range of states for a connected SoundInput
    // Returns the span of states to reset in all inputs
    fn reset_states(&self, dst_input: SoundInputId, span: GridSpan) -> GridSpan;
}

pub struct DynamicSoundProcessorData<T: SoundState> {
    id: SoundProcessorId,
    state_table: RwLock<StateTable<T>>,
    state_partition: RwLock<StateTablePartition>,
}

impl<T: SoundState> DynamicSoundProcessorData<T> {
    pub(super) fn new(id: SoundProcessorId) -> DynamicSoundProcessorData<T> {
        DynamicSoundProcessorData {
            id,
            state_table: RwLock::new(StateTable::new()),
            state_partition: RwLock::new(StateTablePartition::new()),
        }
    }

    pub fn id(&self) -> SoundProcessorId {
        self.id
    }

    fn num_states(&self) -> usize {
        assert_eq!(
            self.state_partition.read().total_size(),
            self.state_table.read().total_size()
        );
        self.state_table.read().total_size()
    }

    fn find_state_index(&self, dst_input: SoundInputId, dst_state_index: usize) -> usize {
        self.state_partition
            .read()
            .get_index(dst_input, dst_state_index)
    }

    pub(super) fn get_state(&self, state_index: usize) -> StateTableLock<T> {
        StateTableLock::new(self.state_table.read(), state_index)
    }

    fn add_dst(&self, dst_input: SoundInputId) {
        assert_eq!(
            self.state_partition.read().total_size(),
            self.state_table.read().total_size()
        );
        self.state_partition.write().add_dst(dst_input);
        assert_eq!(
            self.state_partition.read().total_size(),
            self.state_table.read().total_size()
        );
    }

    fn remove_dst(&self, dst_input: SoundInputId) {
        assert_eq!(
            self.state_partition.read().total_size(),
            self.state_table.read().total_size()
        );
        self.state_partition.write().remove_dst(dst_input);
        assert_eq!(
            self.state_partition.read().total_size(),
            self.state_table.read().total_size()
        );
    }

    fn insert_dst_states(&self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        let s = self.state_partition.write().add_dst_states(dst_input, span);
        self.state_table.write().insert_states(s);
        s
    }

    fn erase_dst_states(&self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        let s = self
            .state_partition
            .write()
            .remove_dst_states(dst_input, span);
        self.state_table.write().erase_states(s);
        s
    }

    fn reset_states(&self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        let s = self.state_partition.read().get_span(dst_input, span);
        self.state_table.read().reset_states(s);
        s
    }
}

pub struct WrappedDynamicSoundProcessor<T: DynamicSoundProcessor> {
    instance: Arc<T>,
    data: Arc<DynamicSoundProcessorData<T::StateType>>,
}

impl<T: DynamicSoundProcessor> WrappedDynamicSoundProcessor<T> {
    pub fn new(
        instance: Arc<T>,
        data: Arc<DynamicSoundProcessorData<T::StateType>>,
    ) -> WrappedDynamicSoundProcessor<T> {
        WrappedDynamicSoundProcessor { instance, data }
    }

    pub fn instance(&self) -> &T {
        &self.instance
    }

    pub fn id(&self) -> SoundProcessorId {
        self.data.id
    }

    pub(super) fn get_state<'a>(&'a self, index: usize) -> StateTableLock<'a, T::StateType> {
        StateTableLock::new(self.data.state_table.read(), index)
    }
}

impl<T: DynamicSoundProcessor> SoundProcessorWrapper for WrappedDynamicSoundProcessor<T> {
    fn id(&self) -> SoundProcessorId {
        self.data.id
    }

    fn process_audio(&self, output_buffer: &mut SoundChunk, context: Context) {
        let table = self.data.state_table.read();
        let f = context.current_frame().into_processor_frame().unwrap();
        let state = table.get_state(f.state_index);
        let sc = ProcessorContext::new(output_buffer, state, context);
        self.instance.process_audio(sc);
    }

    fn is_static(&self) -> bool {
        false
    }

    fn on_start_processing(&self) {}

    fn on_stop_processing(&self) {}

    fn add_dst(&self, dst_input: SoundInputId) {
        self.data.add_dst(dst_input);
    }

    fn erase_dst_states(&self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        self.data.erase_dst_states(dst_input, span)
    }

    fn insert_dst_states(&self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        self.data.insert_dst_states(dst_input, span)
    }

    fn find_state_index(&self, dst_input: SoundInputId, dst_state_index: usize) -> usize {
        self.data.find_state_index(dst_input, dst_state_index)
    }

    fn num_states(&self) -> usize {
        self.data.num_states()
    }

    fn produces_output(&self) -> bool {
        true
    }

    fn remove_dst(&self, dst_input: SoundInputId) {
        self.data.remove_dst(dst_input)
    }

    fn reset_states(&self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        self.data.reset_states(dst_input, span)
    }
}

struct StaticInputStates {
    input_id: SoundInputId,
    num_states: usize,
}

pub struct StaticSoundProcessorData<T: SoundState> {
    id: SoundProcessorId,
    state: RwLock<T>,
    dst_inputs: RwLock<Vec<StaticInputStates>>,
}

impl<T: SoundState> StaticSoundProcessorData<T> {
    pub(super) fn new(id: SoundProcessorId) -> StaticSoundProcessorData<T> {
        StaticSoundProcessorData {
            id,
            state: RwLock::new(T::default()),
            dst_inputs: RwLock::new(Vec::new()),
        }
    }

    pub fn id(&self) -> SoundProcessorId {
        self.id
    }

    fn find_state_index(&self, dst_input: SoundInputId, dst_state_index: usize) -> usize {
        assert!(match self
            .dst_inputs
            .read()
            .iter()
            .find(|is| is.input_id == dst_input)
        {
            Some(is) => is.num_states == 1,
            None => false,
        });
        assert_eq!(dst_state_index, 0);
        0
    }

    pub(super) fn get_state(&self) -> &RwLock<T> {
        &self.state
    }

    fn add_dst(&self, dst_input: SoundInputId) {
        assert!(self
            .dst_inputs
            .read()
            .iter()
            .find(|is| is.input_id == dst_input)
            .is_none());
        self.dst_inputs.write().push(StaticInputStates {
            input_id: dst_input,
            num_states: 0,
        });
    }

    fn remove_dst(&self, dst_input: SoundInputId) {
        assert_eq!(
            self.dst_inputs
                .read()
                .iter()
                .filter(|is| is.input_id == dst_input)
                .count(),
            1
        );
        let i = self
            .dst_inputs
            .read()
            .iter()
            .position(|is| is.input_id == dst_input)
            .unwrap();
        let states = self.dst_inputs.write().remove(i);
        assert_eq!(states.num_states, 0);
    }

    fn insert_dst_states(&self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        assert_eq!(
            self.dst_inputs
                .read()
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
            .read()
            .iter()
            .position(|is| is.input_id == dst_input)
            .unwrap();
        let si = &mut self.dst_inputs.write()[i];
        if si.num_states == 1 {
            panic!();
        }
        si.num_states = 1;
        GridSpan::new_empty()
    }

    fn erase_dst_states(&self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        assert_eq!(
            self.dst_inputs
                .read()
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
            .read()
            .iter()
            .position(|is| is.input_id == dst_input)
            .unwrap();
        let si = &mut self.dst_inputs.write()[i];
        if si.num_states == 0 {
            panic!();
        }
        si.num_states = 0;
        GridSpan::new_empty()
    }
}

pub struct WrappedStaticSoundProcessor<T: StaticSoundProcessor> {
    instance: Arc<T>,
    data: Arc<StaticSoundProcessorData<T::StateType>>,
}

impl<T: StaticSoundProcessor> WrappedStaticSoundProcessor<T> {
    pub fn new(
        instance: Arc<T>,
        data: Arc<StaticSoundProcessorData<T::StateType>>,
    ) -> WrappedStaticSoundProcessor<T> {
        WrappedStaticSoundProcessor { instance, data }
    }

    pub fn instance(&self) -> &T {
        &self.instance
    }

    pub fn id(&self) -> SoundProcessorId {
        self.data.id
    }

    pub(super) fn get_state(&self) -> &RwLock<T::StateType> {
        &self.data.state
    }
}

// A static sound processor allows any number of sound inputs to be connected, but all
// will receive copies of the same single audio stream, and all may have at most one
// state.
impl<T: StaticSoundProcessor> SoundProcessorWrapper for WrappedStaticSoundProcessor<T> {
    fn id(&self) -> SoundProcessorId {
        self.data.id
    }

    fn process_audio(&self, output_buffer: &mut SoundChunk, context: Context) {
        let state = &self.data.state;
        let sc = ProcessorContext::new(output_buffer, state, context);
        self.instance.process_audio(sc);
    }

    fn on_start_processing(&self) {
        self.instance.on_start_processing();
    }

    fn on_stop_processing(&self) {
        self.instance.on_stop_processing();
    }

    fn is_static(&self) -> bool {
        true
    }

    fn num_states(&self) -> usize {
        1
    }

    fn produces_output(&self) -> bool {
        self.instance.produces_output()
    }

    fn reset_states(&self, _dst_input: SoundInputId, _span: GridSpan) -> GridSpan {
        // no-op, static sound sources can't be reset
        GridSpan::new_empty()
    }

    fn add_dst(&self, dst_input: SoundInputId) {
        assert!(self.produces_output());
        self.data.add_dst(dst_input);
    }

    fn remove_dst(&self, dst_input: SoundInputId) {
        assert!(self.produces_output());
        self.data.remove_dst(dst_input)
    }

    fn insert_dst_states(&self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        assert!(self.produces_output());
        self.data.insert_dst_states(dst_input, span)
    }

    fn erase_dst_states(&self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        assert!(self.produces_output());
        self.data.erase_dst_states(dst_input, span)
    }

    fn find_state_index(&self, dst_input: SoundInputId, dst_state_index: usize) -> usize {
        self.data.find_state_index(dst_input, dst_state_index)
    }
}
