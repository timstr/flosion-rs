use std::marker::PhantomData;

use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::core::{
    engine::{compiledprocessor::CompiledSoundInputNode, soundgraphcompiler::SoundGraphCompiler},
    sound::{
        argument::ArgumentScope,
        context::AudioContext,
        soundinput::{
            InputContext, ProcessorInput, SoundInputBackend, SoundInputCategory, SoundInputLocation,
        },
        soundprocessor::{
            CompiledComponentVisitor, CompiledProcessorComponent, SoundProcessorId, StartOver,
        },
    },
    soundchunk::{SoundChunk, CHUNK_SIZE},
    stashing::{StashingContext, UnstashingContext},
};

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum KeyReuse {
    FinishOldCancelNew,
    StopOldStartNew,
}

enum KeyDuration {
    Forever,
    Samples(usize),
}

struct KeyPlayingData<S> {
    id: usize,
    age: usize,
    duration: KeyDuration,
    state: S,
}

enum QueuedKeyState<S> {
    NotPlaying,
    Playing(KeyPlayingData<S>),
}

pub struct KeyedInputQueueBackend<S> {
    num_keys: usize,
    phantom_data: PhantomData<S>,
}

impl<S> KeyedInputQueueBackend<S> {
    pub fn new(num_keys: usize) -> KeyedInputQueueBackend<S> {
        KeyedInputQueueBackend {
            num_keys,
            phantom_data: PhantomData,
        }
    }

    pub fn num_keys(&self) -> usize {
        self.num_keys
    }

    pub fn set_num_keys(&mut self, num_keys: usize) {
        self.num_keys = num_keys;
    }
}

impl<S: Send> SoundInputBackend for KeyedInputQueueBackend<S> {
    type CompiledType<'ctx> = CompiledKeyedInputQueue<'ctx, S>;

    fn category(&self) -> SoundInputCategory {
        SoundInputCategory::Branched(self.num_keys)
    }

    fn compile<'ctx>(
        &self,
        location: SoundInputLocation,
        target: Option<SoundProcessorId>,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx> {
        CompiledKeyedInputQueue {
            items: (0..self.num_keys)
                .map(|_| CompiledKeyedInputQueueItem {
                    node: CompiledSoundInputNode::new(
                        location,
                        compiler.compile_sound_processor(target),
                    ),
                    state: QueuedKeyState::NotPlaying,
                })
                .collect(),
        }
    }
}

impl<S> Stashable<StashingContext> for KeyedInputQueueBackend<S> {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.u64(self.num_keys as _);
    }
}

impl<S> UnstashableInplace<UnstashingContext<'_>> for KeyedInputQueueBackend<S> {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext<'_>>,
    ) -> Result<(), UnstashError> {
        let n = unstasher.u64_always()?;
        if unstasher.time_to_write() {
            self.num_keys = n as _;
        }
        Ok(())
    }
}

struct CompiledKeyedInputQueueItem<'ctx, S> {
    node: CompiledSoundInputNode<'ctx>,
    state: QueuedKeyState<S>,
}

pub struct CompiledKeyedInputQueue<'ctx, S> {
    items: Vec<CompiledKeyedInputQueueItem<'ctx, S>>,
}

impl<'ctx, S> CompiledKeyedInputQueue<'ctx, S> {
    pub fn start_key(
        &mut self,
        duration_samples: Option<usize>,
        id: usize,
        state: S,
        reuse: KeyReuse,
    ) {
        let mut oldest_key_index_and_age = None;
        let mut available_index = None;
        for (i, d) in self.items.iter_mut().enumerate() {
            if let QueuedKeyState::Playing(key_data) = &mut d.state {
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

        let data = &mut self.items[index];

        data.node.start_over_at(0); // TODO: sample offset
        let key_data = KeyPlayingData {
            id,
            state,
            age: 0,
            duration: match duration_samples {
                Some(s) => KeyDuration::Samples(s),
                None => KeyDuration::Forever,
            },
        };
        data.state = QueuedKeyState::Playing(key_data);
    }

    pub fn release_key(&mut self, id: usize) {
        for d in &mut self.items {
            if let QueuedKeyState::Playing(key_data) = &mut d.state {
                if key_data.id == id {
                    key_data.duration = KeyDuration::Samples(0);
                }
            }
        }
    }

    pub fn release_all_keys(&mut self) {
        for d in &mut self.items {
            if let QueuedKeyState::Playing(key_data) = &mut d.state {
                key_data.duration = KeyDuration::Samples(0);
            }
        }
    }

    pub fn step_active_keys<'a, F: FnMut(&mut S, InputContext<'a>) -> InputContext<'a>>(
        &mut self,
        dst: &mut SoundChunk,
        context: &'a AudioContext<'a>,
        mut f: F,
    ) {
        // TODO: allow per-key chunk sample offsets, store remaining chunk in state

        dst.silence();
        let mut temp_chunk = SoundChunk::new();
        for d in &mut self.items {
            if let QueuedKeyState::Playing(key_data) = &mut d.state {
                // TODO: allow keys to stack (after ignoring key repeats in keyboard_ui)
                if let KeyDuration::Samples(s) = &mut key_data.duration {
                    if *s < CHUNK_SIZE {
                        d.node.timing_mut().request_release(*s);
                        *s = 0;
                    } else {
                        *s -= CHUNK_SIZE;
                    }
                }

                d.node.step(
                    &mut temp_chunk,
                    f(&mut key_data.state, InputContext::new(context)),
                );

                key_data.age += 1;
                if d.node.timing().is_done() {
                    d.state = QueuedKeyState::NotPlaying;
                }

                // TODO: how to make this adjustable?
                slicemath::mul_scalar_inplace(&mut temp_chunk.l, 0.1);
                slicemath::mul_scalar_inplace(&mut temp_chunk.r, 0.1);
                slicemath::add_inplace(&mut dst.l, &temp_chunk.l);
                slicemath::add_inplace(&mut dst.r, &temp_chunk.r);
            }
        }
    }
}

impl<'ctx, S> CompiledProcessorComponent for CompiledKeyedInputQueue<'ctx, S> {
    fn visit(&self, visitor: &mut dyn CompiledComponentVisitor) {
        for item in &self.items {
            visitor.input_node(&item.node);
        }
    }
}

impl<'ctx, S> StartOver for CompiledKeyedInputQueue<'ctx, S> {
    fn start_over(&mut self) {
        for item in &mut self.items {
            item.state = QueuedKeyState::NotPlaying
        }
    }
}

pub type KeyedInputQueue<S> = ProcessorInput<KeyedInputQueueBackend<S>>;

impl<S> KeyedInputQueue<S> {
    pub fn new(num_keys: usize, argument_scope: ArgumentScope) -> KeyedInputQueue<S> {
        ProcessorInput::new_from_parts(
            argument_scope,
            KeyedInputQueueBackend {
                num_keys,
                phantom_data: PhantomData,
            },
        )
    }
}
