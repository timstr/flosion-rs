use std::marker::PhantomData;

use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::core::{
    engine::{soundgraphcompiler::SoundGraphCompiler, stategraphnode::CompiledSoundInputBranch},
    sound::{
        argument::ArgumentScope,
        context::Context,
        soundinput::{BasicProcessorInput, InputContext, InputOptions, ProcessorInputId},
        soundprocessor::{
            ProcessorComponent, ProcessorComponentVisitor, ProcessorComponentVisitorMut,
            SoundProcessorId, StartOver,
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

pub struct KeyedInputQueue<S> {
    input: BasicProcessorInput,
    phantom_data: PhantomData<S>,
}

impl<S> KeyedInputQueue<S> {
    pub fn new(options: InputOptions, num_keys: usize, scope: ArgumentScope) -> KeyedInputQueue<S> {
        KeyedInputQueue {
            input: BasicProcessorInput::new(options, num_keys, scope),
            phantom_data: PhantomData,
        }
    }

    pub fn id(&self) -> ProcessorInputId {
        self.input.id()
    }

    pub fn num_keys(&self) -> usize {
        self.input.branches()
    }

    pub fn set_num_keys(&mut self, num_keys: usize) {
        self.input.set_branches(num_keys);
    }
}

impl<S: Send> ProcessorComponent for KeyedInputQueue<S> {
    type CompiledType<'ctx> = CompiledKeyedInputQueue<'ctx, S>;

    fn visit<'a>(&self, visitor: &'a mut dyn ProcessorComponentVisitor) {
        visitor.input(&self.input);
    }

    fn visit_mut<'a>(&mut self, visitor: &'a mut dyn ProcessorComponentVisitorMut) {
        visitor.input(&mut self.input);
    }

    fn compile<'ctx>(
        &self,
        processor_id: SoundProcessorId,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx> {
        CompiledKeyedInputQueue {
            items: (0..self.input.branches())
                .map(|_| CompiledKeyedInputQueueItem {
                    input: self.input.compile_branch(processor_id, compiler),
                    state: QueuedKeyState::NotPlaying,
                })
                .collect(),
        }
    }
}

impl<S> Stashable<StashingContext> for KeyedInputQueue<S> {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        self.input.stash(stasher);
    }
}

impl<S> UnstashableInplace<UnstashingContext<'_>> for KeyedInputQueue<S> {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext<'_>>,
    ) -> Result<(), UnstashError> {
        self.input.unstash_inplace(unstasher)
    }
}

struct CompiledKeyedInputQueueItem<'ctx, S> {
    input: CompiledSoundInputBranch<'ctx>,
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

        data.input.start_over_at(0); // TODO: sample offset
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
        context: &'a Context<'a>,
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
                        d.input.timing_mut().request_release(*s);
                        *s = 0;
                    } else {
                        *s -= CHUNK_SIZE;
                    }
                }

                d.input.step(
                    &mut temp_chunk,
                    f(&mut key_data.state, InputContext::new(context)),
                );

                key_data.age += 1;
                if d.input.timing().is_done() {
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

impl<'ctx, S> StartOver for CompiledKeyedInputQueue<'ctx, S> {
    fn start_over(&mut self) {
        for item in &mut self.items {
            item.state = QueuedKeyState::NotPlaying
        }
    }
}
