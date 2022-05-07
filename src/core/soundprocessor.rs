use std::sync::Arc;

use parking_lot::RwLock;

use super::{
    context::{Context, ProcessorContext},
    graphobject::{ObjectWrapper, WithObjectType},
    gridspan::GridSpan,
    soundchunk::SoundChunk,
    soundinput::SoundInputId,
    soundprocessortools::SoundProcessorTools,
    soundstate::SoundState,
    statetable::{StateTable, StateTableLock, StateTablePartition},
    uniqueid::UniqueId,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SoundProcessorId(pub usize);

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

pub trait DynamicSoundProcessor: 'static + Sync + Send + WithObjectType {
    type StateType: SoundState;
    fn new(tools: &mut SoundProcessorTools<'_, Self::StateType>) -> Self
    where
        Self: Sized;
    fn process_audio(&self, dst: &mut SoundChunk, context: ProcessorContext<'_, Self::StateType>);
}

pub trait StaticSoundProcessor: 'static + Sync + Send + WithObjectType {
    type StateType: SoundState;
    fn new(tools: &mut SoundProcessorTools<'_, Self::StateType>) -> Self
    where
        Self: Sized;
    fn process_audio(&self, dst: &mut SoundChunk, context: ProcessorContext<'_, Self::StateType>);
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

pub struct SoundProcessorData<T: SoundState> {
    id: SoundProcessorId,
    state_table: RwLock<StateTable<T>>,
    state_partition: RwLock<StateTablePartition>,
}

impl<T: SoundState> SoundProcessorData<T> {
    pub(super) fn new(id: SoundProcessorId, is_static: bool) -> SoundProcessorData<T> {
        let mut state_table = StateTable::new();
        if is_static {
            state_table.insert_states(GridSpan::new_contiguous(0, 1));
        }
        SoundProcessorData {
            id,
            state_table: RwLock::new(state_table),
            state_partition: RwLock::new(StateTablePartition::new(is_static)),
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
    data: Arc<SoundProcessorData<T::StateType>>,
}

impl<T: DynamicSoundProcessor> WrappedDynamicSoundProcessor<T> {
    pub fn new(
        instance: Arc<T>,
        data: Arc<SoundProcessorData<T::StateType>>,
    ) -> WrappedDynamicSoundProcessor<T> {
        WrappedDynamicSoundProcessor { instance, data }
    }

    pub fn instance(&self) -> &T {
        &self.instance
    }

    pub fn id(&self) -> SoundProcessorId {
        self.data.id
    }

    pub fn num_states(&self) -> usize {
        self.data.num_states()
    }

    pub(super) fn data(&self) -> &Arc<SoundProcessorData<T::StateType>> {
        &self.data
    }
}

impl<T: DynamicSoundProcessor> Clone for WrappedDynamicSoundProcessor<T> {
    fn clone(&self) -> Self {
        Self {
            instance: self.instance.clone(),
            data: self.data.clone(),
        }
    }
}

impl<T: DynamicSoundProcessor> SoundProcessorWrapper for WrappedDynamicSoundProcessor<T> {
    fn id(&self) -> SoundProcessorId {
        self.data.id
    }

    fn process_audio(&self, dst: &mut SoundChunk, context: Context) {
        let table = self.data.state_table.read();
        let f = context.current_frame().into_processor_frame().unwrap();
        let state = table.get_state(f.state_index);
        let sc = ProcessorContext::new(state, f.state_index, context);
        self.instance.process_audio(dst, sc);
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

impl<T: DynamicSoundProcessor> ObjectWrapper for WrappedDynamicSoundProcessor<T> {
    type Type = T;

    fn get_object(&self) -> &T {
        &*self.instance
    }
}

pub struct WrappedStaticSoundProcessor<T: StaticSoundProcessor> {
    instance: Arc<T>,
    data: Arc<SoundProcessorData<T::StateType>>,
}

impl<T: StaticSoundProcessor> WrappedStaticSoundProcessor<T> {
    pub fn new(
        instance: Arc<T>,
        data: Arc<SoundProcessorData<T::StateType>>,
    ) -> WrappedStaticSoundProcessor<T> {
        debug_assert!(data.num_states() == 1);
        WrappedStaticSoundProcessor { instance, data }
    }

    pub fn instance(&self) -> &T {
        &self.instance
    }

    pub fn id(&self) -> SoundProcessorId {
        self.data.id
    }

    pub(super) fn data(&self) -> &Arc<SoundProcessorData<T::StateType>> {
        &self.data
    }
}

impl<T: StaticSoundProcessor> Clone for WrappedStaticSoundProcessor<T> {
    fn clone(&self) -> Self {
        Self {
            instance: self.instance.clone(),
            data: self.data.clone(),
        }
    }
}

// A static sound processor allows any number of sound inputs to be connected, but all
// will receive copies of the same single audio stream, and all may have at most one
// state.
impl<T: StaticSoundProcessor> SoundProcessorWrapper for WrappedStaticSoundProcessor<T> {
    fn id(&self) -> SoundProcessorId {
        self.data.id
    }

    fn process_audio(&self, dst: &mut SoundChunk, context: Context) {
        let table = self.data.state_table.read();
        debug_assert!(
            context
                .current_frame()
                .into_processor_frame()
                .unwrap()
                .state_index
                == 0
        );
        let state = table.get_state(0);
        let sc = ProcessorContext::new(state, 0, context);
        self.instance.process_audio(dst, sc);
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
        debug_assert!(self.produces_output());
        self.data.add_dst(dst_input);
    }

    fn remove_dst(&self, dst_input: SoundInputId) {
        debug_assert!(self.produces_output());
        self.data.remove_dst(dst_input)
    }

    fn insert_dst_states(&self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        debug_assert!(self.produces_output());
        self.data.insert_dst_states(dst_input, span)
    }

    fn erase_dst_states(&self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        debug_assert!(self.produces_output());
        self.data.erase_dst_states(dst_input, span)
    }

    fn find_state_index(&self, dst_input: SoundInputId, dst_state_index: usize) -> usize {
        self.data.find_state_index(dst_input, dst_state_index)
    }
}

impl<T: StaticSoundProcessor> ObjectWrapper for WrappedStaticSoundProcessor<T> {
    type Type = T;

    fn get_object(&self) -> &T {
        &*self.instance
    }
}
