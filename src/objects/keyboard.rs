use std::sync::atomic::{AtomicI16, Ordering};

use atomic_float::AtomicF32;

use crate::core::{
    context::Context,
    graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    key::Key,
    numbersource::StateNumberSourceHandle,
    numeric,
    soundchunk::SoundChunk,
    soundinput::InputOptions,
    soundprocessor::{SoundProcessor, StreamStatus},
    soundprocessortools::SoundProcessorTools,
    statetree::{KeyedInput, KeyedInputNode, NoState, ProcessorState},
};

const INVALID_ID: i16 = -1;
const KEY_PLAYING: i16 = -2;
const KEY_NOT_PLAYING: i16 = -3;
const KEY_RELEASED: i16 = -4;

pub struct KeyboardKey {
    frequency: AtomicF32,
    id: AtomicI16,
    status: AtomicI16,
}

impl KeyboardKey {
    fn new() -> Self {
        Self {
            frequency: AtomicF32::new(f32::NAN),
            id: AtomicI16::new(INVALID_ID),
            status: AtomicI16::new(KEY_NOT_PLAYING),
        }
    }
}

impl Key for KeyboardKey {}

const MAX_KEYS: usize = 8;

pub struct Keyboard {
    pub input: KeyedInput<KeyboardKey, NoState>,
    pub key_frequency: StateNumberSourceHandle,
}

impl Keyboard {
    pub fn press_key(&self, key_id: u16, frequency: f32) {
        for ks in self.input.keys() {
            if ks.id.load(Ordering::SeqCst) == (key_id as i16) {
                return;
            }
        }
        for ks in self.input.keys() {
            if ks.id.load(Ordering::SeqCst) == INVALID_ID {
                ks.id.store(key_id as i16, Ordering::SeqCst);
                ks.status.store(KEY_PLAYING, Ordering::SeqCst);
                ks.frequency.store(frequency, Ordering::SeqCst);
                return;
            }
        }
    }

    pub fn release_key(&self, key_id: u16) {
        for ks in self.input.keys() {
            if ks.id.load(Ordering::SeqCst) == (key_id as i16) {
                ks.status.store(KEY_RELEASED, Ordering::SeqCst);
                // ks.frequency.store(f32::NAN, Ordering::SeqCst);
                return;
            }
        }
    }
}

impl SoundProcessor for Keyboard {
    const IS_STATIC: bool = true;

    type StateType = NoState;

    type InputType = KeyedInput<KeyboardKey, NoState>;

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Self {
        let input = KeyedInput::new(
            InputOptions {
                interruptible: true,
                realtime: true,
            },
            &mut tools,
            (0..MAX_KEYS).map(|_| KeyboardKey::new()).collect(),
        );
        let key_frequency = input.add_number_source(&mut tools, |dst, k, _s| {
            numeric::fill(dst, k.frequency.load(Ordering::Relaxed));
        });
        Keyboard {
            input,
            key_frequency,
        }
    }

    fn get_input(&self) -> &Self::InputType {
        &self.input
    }

    fn make_state(&self) -> Self::StateType {
        NoState {}
    }

    fn process_audio(
        state: &mut ProcessorState<NoState>,
        input: &mut KeyedInputNode<KeyboardKey, NoState>,
        dst: &mut SoundChunk,
        ctx: Context,
    ) -> StreamStatus {
        dst.silence();
        let mut scratch_buffer = SoundChunk::new();
        scratch_buffer.silence();
        for kd in input.data_mut() {
            let key_id = kd.key().id.load(Ordering::SeqCst);
            if key_id == INVALID_ID {
                continue;
            }
            let status = kd.key().status.load(Ordering::SeqCst);
            debug_assert!(status != KEY_NOT_PLAYING);
            if kd.needs_reset() {
                // TODO: gather fine timing data and apply it here
                kd.reset(0);
            }
            if status == KEY_RELEASED && !kd.was_released() {
                // TODO: gather fine timing data and apply it here
                kd.request_release(0);
            }
            kd.step(state, &mut scratch_buffer, &ctx);
            // TODO: create FMA functions
            numeric::mul_scalar_inplace(&mut scratch_buffer.l, 0.1);
            numeric::mul_scalar_inplace(&mut scratch_buffer.r, 0.1);
            numeric::add_inplace(&mut dst.l, &scratch_buffer.l);
            numeric::add_inplace(&mut dst.r, &scratch_buffer.r);

            // TODO: prevent inputs from playing forever if they don't respond to release requests
            if kd.is_done() {
                kd.key().id.store(INVALID_ID, Ordering::SeqCst);
                kd.key().status.store(KEY_NOT_PLAYING, Ordering::SeqCst);
                kd.require_reset();
            }
        }
        StreamStatus::Playing
    }
}

impl WithObjectType for Keyboard {
    const TYPE: ObjectType = ObjectType::new("keyboard");
}
