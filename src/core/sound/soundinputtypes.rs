use std::{any::Any, marker::PhantomData};

use parking_lot::RwLock;

use crate::core::{
    anydata::AnyData,
    engine::{
        nodegen::NodeGen,
        soundinputnode::{SoundInputNode, SoundProcessorInput},
        stategraphnode::NodeTarget,
    },
    soundchunk::{SoundChunk, CHUNK_SIZE},
};

use super::{
    context::{Context, LocalArrayList},
    soundinput::{InputOptions, InputTiming, SoundInputId},
    soundprocessor::{ProcessorState, StreamStatus},
    soundprocessortools::SoundProcessorTools,
    state::State,
};

pub struct SingleInput {
    id: SoundInputId,
}

impl SingleInput {
    pub fn new(options: InputOptions, tools: &mut SoundProcessorTools) -> SingleInput {
        SingleInput {
            id: tools.add_sound_input(options, /*num_keys=*/ 1),
        }
    }

    pub fn id(&self) -> SoundInputId {
        self.id
    }
}

impl SoundProcessorInput for SingleInput {
    type NodeType<'ctx> = SingleInputNode<'ctx>;

    fn make_node<'a, 'ctx>(&self, nodegen: &mut NodeGen<'a, 'ctx>) -> Self::NodeType<'ctx> {
        SingleInputNode::new(self.id, nodegen)
    }

    fn list_ids(&self) -> Vec<SoundInputId> {
        vec![self.id]
    }
}

pub struct SingleInputNode<'ctx> {
    target: NodeTarget<'ctx>,
}

impl<'ctx> SingleInputNode<'ctx> {
    pub fn new<'a>(id: SoundInputId, nodegen: &mut NodeGen<'a, 'ctx>) -> SingleInputNode<'ctx> {
        SingleInputNode {
            target: NodeTarget::new(id, 0, nodegen),
        }
    }

    pub fn timing(&self) -> &InputTiming {
        self.target.timing()
    }

    pub fn timing_mut(&mut self) -> &mut InputTiming {
        self.target.timing_mut()
    }

    pub fn step<T: ProcessorState>(
        &mut self,
        processor_state: &T,
        dst: &mut SoundChunk,
        ctx: &Context,
        local_arrays: LocalArrayList,
    ) -> StreamStatus {
        self.target
            .step(processor_state, dst, ctx, AnyData::new(&()), local_arrays)
    }

    pub fn reset(&mut self, sample_offset: usize) {
        self.target.reset(sample_offset);
    }
}

impl<'ctx> SoundInputNode<'ctx> for SingleInputNode<'ctx> {
    fn targets(&self) -> &[NodeTarget<'ctx>] {
        std::slice::from_ref(&self.target)
    }

    fn targets_mut(&mut self) -> &mut [NodeTarget<'ctx>] {
        std::slice::from_mut(&mut self.target)
    }
}

pub struct KeyedInput<S: State + Default> {
    id: SoundInputId,
    num_keys: usize,
    phantom_data: PhantomData<S>,
}

impl<S: State + Default> KeyedInput<S> {
    pub fn new(options: InputOptions, tools: &mut SoundProcessorTools, num_keys: usize) -> Self {
        let id = tools.add_sound_input(options, num_keys);
        Self {
            id,
            num_keys,
            phantom_data: PhantomData,
        }
    }

    pub fn id(&self) -> SoundInputId {
        self.id
    }

    // TODO: add/remove keys with SoundProcessorTools
}

impl<S: State + Default> SoundProcessorInput for KeyedInput<S> {
    type NodeType<'ctx> = KeyedInputNode<'ctx, S>;

    fn make_node<'a, 'ctx>(&self, nodegen: &mut NodeGen<'a, 'ctx>) -> Self::NodeType<'ctx> {
        KeyedInputNode {
            id: self.id,
            targets: (0..self.num_keys)
                .map(|i| NodeTarget::new(self.id, i, nodegen))
                .collect(),
            states: (0..self.num_keys).map(|_| S::default()).collect(),
        }
    }

    fn list_ids(&self) -> Vec<SoundInputId> {
        vec![self.id]
    }
}

pub struct KeyedInputData<'ctx, S: State + Default> {
    id: SoundInputId,
    target: NodeTarget<'ctx>,
    state: S,
}

pub struct KeyedInputNode<'ctx, S: State + Default> {
    id: SoundInputId,
    targets: Vec<NodeTarget<'ctx>>,
    states: Vec<S>,
}

pub struct KeyedInputNodeItem<'a, 'ctx, S> {
    target: &'a mut NodeTarget<'ctx>,
    state: &'a mut S,
}

impl<'a, 'ctx, S: 'static> KeyedInputNodeItem<'a, 'ctx, S> {
    pub fn timing(&self) -> &InputTiming {
        self.target.timing()
    }

    pub fn timing_mut(&mut self) -> &mut InputTiming {
        self.target.timing_mut()
    }

    pub fn step<T: ProcessorState>(
        &mut self,
        processor_state: &T,
        dst: &mut SoundChunk,
        ctx: &Context,
        local_arrays: LocalArrayList,
    ) -> StreamStatus {
        self.target.step(
            processor_state,
            dst,
            ctx,
            AnyData::new(self.state),
            local_arrays,
        )
    }

    pub fn state(&self) -> &S {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut S {
        &mut self.state
    }

    pub fn reset(&mut self, sample_offset: usize) {
        self.target.reset(sample_offset);
    }
}

impl<'ctx, S: State + Default> KeyedInputNode<'ctx, S> {
    pub fn items_mut<'a>(&'a mut self) -> impl Iterator<Item = KeyedInputNodeItem<'a, 'ctx, S>> {
        self.targets
            .iter_mut()
            .zip(self.states.iter_mut())
            .map(|(t, s)| KeyedInputNodeItem {
                target: t,
                state: s,
            })
    }

    pub fn states(&self) -> &[S] {
        &self.states
    }

    pub fn states_mut(&mut self) -> &mut [S] {
        &mut self.states
    }
}

impl<'ctx, S: State + Default> SoundInputNode<'ctx> for KeyedInputNode<'ctx, S> {
    fn targets(&self) -> &[NodeTarget<'ctx>] {
        &self.targets
    }

    fn targets_mut(&mut self) -> &mut [NodeTarget<'ctx>] {
        &mut self.targets
    }
}

impl SoundProcessorInput for () {
    type NodeType<'ctx> = ();

    fn make_node<'a, 'ctx>(&self, _nodegen: &mut NodeGen<'a, 'ctx>) -> Self::NodeType<'ctx> {
        ()
    }

    fn list_ids(&self) -> Vec<SoundInputId> {
        Vec::new()
    }
}

pub struct SingleInputList {
    // NOTE: this RwLock is mostly a formality, since
    // SoundProcessorTools is required to change the input
    // anyway and therefore mutable access to the topology
    // is already held
    input_ids: RwLock<Vec<SoundInputId>>,
    options: InputOptions,
}

impl SingleInputList {
    pub fn new(
        count: usize,
        options: InputOptions,
        tools: &mut SoundProcessorTools,
    ) -> SingleInputList {
        SingleInputList {
            input_ids: RwLock::new(
                (0..count)
                    .map(|_| tools.add_sound_input(options, /*num_keys=*/ 1))
                    .collect(),
            ),
            options,
        }
    }

    pub fn add_input(&self, tools: &mut SoundProcessorTools) {
        self.input_ids
            .write()
            .push(tools.add_sound_input(self.options, /*num_keys=*/ 1));
    }

    pub fn remove_input(&self, id: SoundInputId, tools: &mut SoundProcessorTools) {
        let mut input_ids = self.input_ids.write();
        assert!(input_ids.iter().filter(|i| **i == id).count() == 1);
        tools.remove_sound_input(id, tools.processor_id());
        input_ids.retain(|i| *i != id);
    }

    pub fn get_input_ids(&self) -> Vec<SoundInputId> {
        self.input_ids.read().clone()
    }

    pub fn length(&self) -> usize {
        self.input_ids.read().len()
    }
}

impl SoundProcessorInput for SingleInputList {
    type NodeType<'ctx> = SingleInputListNode<'ctx>;

    fn make_node<'a, 'ctx>(&self, nodegen: &mut NodeGen<'a, 'ctx>) -> Self::NodeType<'ctx> {
        SingleInputListNode {
            targets: self
                .input_ids
                .read()
                .iter()
                .map(|id| NodeTarget::new(*id, 0, nodegen))
                .collect(),
        }
    }

    fn list_ids(&self) -> Vec<SoundInputId> {
        self.input_ids.read().clone()
    }
}

pub struct SingleInputListNode<'ctx> {
    targets: Vec<NodeTarget<'ctx>>,
}

pub struct SingleInputListNodeItem<'a, 'ctx> {
    target: &'a mut NodeTarget<'ctx>,
}

impl<'a, 'ctx> SingleInputListNodeItem<'a, 'ctx> {
    pub fn timing(&self) -> &InputTiming {
        self.target.timing()
    }

    pub fn timing_mut(&mut self) -> &mut InputTiming {
        self.target.timing_mut()
    }

    pub fn step<T: ProcessorState>(
        &mut self,
        processor_state: &T,
        dst: &mut SoundChunk,
        ctx: &Context,
        local_arrays: LocalArrayList,
    ) -> StreamStatus {
        self.target
            .step(processor_state, dst, ctx, AnyData::new(&()), local_arrays)
    }

    pub fn reset(&mut self, sample_offset: usize) {
        self.target.reset(sample_offset)
    }
}

impl<'ctx> SingleInputListNode<'ctx> {
    pub fn items_mut<'a>(&'a mut self) -> impl Iterator<Item = SingleInputListNodeItem<'a, 'ctx>> {
        self.targets
            .iter_mut()
            .map(|t| SingleInputListNodeItem { target: t })
    }
}

impl<'ctx> SoundInputNode<'ctx> for SingleInputListNode<'ctx> {
    fn targets(&self) -> &[NodeTarget<'ctx>] {
        &self.targets
    }

    fn targets_mut(&mut self) -> &mut [NodeTarget<'ctx>] {
        &mut self.targets
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum KeyReuse {
    FinishOldCancelNew,
    StopOldStartNew,
}

enum KeyDuration {
    Forever,
    Samples(usize),
}

struct KeyPlayingData<S: State> {
    id: usize,
    state: S,
    age: usize,
    duration: KeyDuration,
}

enum QueuedKeyState<S: State> {
    NotPlaying(),
    Playing(KeyPlayingData<S>),
}

pub struct KeyedInputQueue<S: State> {
    id: SoundInputId,
    num_keys: usize,
    phantom_data_s: PhantomData<S>,
}

impl<S: State> KeyedInputQueue<S> {
    pub fn new(queue_size: usize, tools: &mut SoundProcessorTools) -> Self {
        let id = tools.add_sound_input(InputOptions::NonSynchronous, queue_size);
        Self {
            id,
            num_keys: queue_size,
            phantom_data_s: PhantomData,
        }
    }

    pub fn id(&self) -> SoundInputId {
        self.id
    }
}

impl<S: State> SoundProcessorInput for KeyedInputQueue<S> {
    type NodeType<'ctx> = KeyedInputQueueNode<'ctx, S>;

    fn make_node<'a, 'ctx>(&self, nodegen: &mut NodeGen<'a, 'ctx>) -> Self::NodeType<'ctx> {
        KeyedInputQueueNode::new(self.id, self.num_keys, nodegen)
    }

    fn list_ids(&self) -> Vec<SoundInputId> {
        vec![self.id]
    }
}

pub struct KeyedInputQueueNode<'ctx, S: State> {
    id: SoundInputId,
    targets: Vec<NodeTarget<'ctx>>,
    states: Vec<QueuedKeyState<S>>,
    active: bool,
}

impl<'ctx, S: State> KeyedInputQueueNode<'ctx, S> {
    fn new<'a>(id: SoundInputId, num_keys: usize, nodegen: &mut NodeGen<'a, 'ctx>) -> Self {
        Self {
            id,
            targets: (0..num_keys)
                .map(|i| NodeTarget::new(id, i, nodegen))
                .collect(),
            states: (0..num_keys)
                .map(|_| QueuedKeyState::NotPlaying())
                .collect(),
            active: false,
        }
    }

    // TODO: add sample_offset in [0, chunk_size)
    // TODO: make stacking optional
    pub fn start_key(
        &mut self,
        duration_samples: Option<usize>,
        id: usize,
        state: S,
        reuse: KeyReuse,
    ) {
        let mut oldest_key_index_and_age = None;
        let mut available_index = None;
        for (i, d) in self.states.iter_mut().enumerate() {
            if let QueuedKeyState::Playing(key_data) = d {
                // if key_data.id == id {
                //     key_data.duration = match duration_samples {
                //         Some(s) => KeyDuration::Samples(s),
                //         None => KeyDuration::Forever,
                //     };
                //     return;
                // }
                oldest_key_index_and_age = match oldest_key_index_and_age {
                    Some((j, s)) => {
                        if key_data.age > s {
                            Some((i, key_data.age))
                        } else {
                            Some((j, s))
                        }
                    }
                    None => Some((i, key_data.age)),
                };
            } else {
                if available_index.is_none() {
                    available_index = Some(i);
                }
            }
        }

        let index = match available_index {
            Some(i) => i,
            None => {
                if reuse == KeyReuse::FinishOldCancelNew {
                    return;
                }
                oldest_key_index_and_age.unwrap().0
            }
        };

        let data = &mut self.targets[index];

        data.reset(0); // TODO: sample offset
        let key_data = KeyPlayingData {
            id,
            state,
            age: 0,
            duration: match duration_samples {
                Some(s) => KeyDuration::Samples(s),
                None => KeyDuration::Forever,
            },
        };
        self.states[index] = QueuedKeyState::Playing(key_data);
    }

    // TODO: add sample_offset in [0, chunk_size)
    pub fn release_key(&mut self, id: usize) {
        for d in &mut self.states {
            if let QueuedKeyState::Playing(key_data) = d {
                if key_data.id == id {
                    key_data.duration = KeyDuration::Samples(0);
                }
            }
        }
    }

    pub fn release_all_keys(&mut self) {
        for d in &mut self.states {
            if let QueuedKeyState::Playing(key_data) = d {
                key_data.duration = KeyDuration::Samples(0);
            }
        }
    }

    pub fn step<T: ProcessorState>(
        &mut self,
        processor_state: &T,
        dst: &mut SoundChunk,
        ctx: &Context,
        local_arrays: LocalArrayList,
    ) -> StreamStatus {
        // TODO: allow per-key chunk sample offsets, store remaining chunk in state

        dst.silence();
        let mut temp_chunk = SoundChunk::new();
        for (d, t) in self.states.iter_mut().zip(self.targets.iter_mut()) {
            if let QueuedKeyState::Playing(key_data) = d {
                // TODO: allow keys to stack (after ignoring key repeats in keyboard_ui)
                if let KeyDuration::Samples(s) = &mut key_data.duration {
                    if *s < CHUNK_SIZE {
                        t.timing_mut().request_release(*s);
                        *s = 0;
                    } else {
                        *s -= CHUNK_SIZE;
                    }
                }

                let a: &dyn Any = &key_data.state;
                t.step(
                    processor_state,
                    &mut temp_chunk,
                    ctx,
                    AnyData::new(a),
                    local_arrays,
                );

                key_data.age += 1;
                if t.timing().is_done() {
                    *d = QueuedKeyState::NotPlaying();
                }

                // TODO: how to make this adjustable?
                numeric::mul_scalar_inplace(&mut temp_chunk.l, 0.1);
                numeric::mul_scalar_inplace(&mut temp_chunk.r, 0.1);
                numeric::add_inplace(&mut dst.l, &temp_chunk.l);
                numeric::add_inplace(&mut dst.r, &temp_chunk.r);
            }
        }
        StreamStatus::Playing
    }
}

impl<'ctx, S: State> SoundInputNode<'ctx> for KeyedInputQueueNode<'ctx, S> {
    fn targets(&self) -> &[NodeTarget<'ctx>] {
        &self.targets
    }

    fn targets_mut(&mut self) -> &mut [NodeTarget<'ctx>] {
        &mut self.targets
    }
}
