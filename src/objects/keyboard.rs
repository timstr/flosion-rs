use std::sync::atomic::{AtomicI16, Ordering};

use atomic_float::AtomicF32;

use crate::core::{
    context::Context,
    graphobject::{ObjectType, WithObjectType},
    key::Key,
    numbersource::StateNumberSourceHandle,
    numeric,
    soundchunk::SoundChunk,
    soundinput::InputOptions,
    soundprocessor::SoundProcessor,
    soundprocessortools::SoundProcessorTools,
    statetree::{KeyedInput, KeyedInputNode, NoState},
};

pub struct KeyboardKey {
    frequency: AtomicF32,
    current_id: AtomicI16,
    previous_id: AtomicI16,
}

impl KeyboardKey {
    fn new() -> Self {
        Self {
            frequency: AtomicF32::new(f32::NAN),
            current_id: AtomicI16::new(-1),
            previous_id: AtomicI16::new(-1),
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
            if ks.current_id.load(Ordering::SeqCst) == (key_id as i16) {
                return;
            }
        }
        for ks in self.input.keys() {
            if ks.current_id.load(Ordering::SeqCst) == -1 {
                ks.current_id.store(key_id as i16, Ordering::SeqCst);
                ks.frequency.store(frequency, Ordering::SeqCst);
                return;
            }
        }
        println!("Warning: keyboard is out of keys to press");
    }

    pub fn release_key(&self, key_id: u16) {
        for ks in self.input.keys() {
            if ks.current_id.load(Ordering::SeqCst) == (key_id as i16) {
                ks.current_id.store(-1, Ordering::SeqCst);
                ks.frequency.store(f32::NAN, Ordering::SeqCst);
                return;
            }
        }
        println!("Warning: keyboard attempted to release a key which was not held");
    }
}

impl SoundProcessor for Keyboard {
    const IS_STATIC: bool = true;

    type StateType = NoState;

    type InputType = KeyedInput<KeyboardKey, NoState>;

    fn new(mut tools: SoundProcessorTools) -> Self {
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
        todo!()
    }

    fn process_audio(
        state: &mut NoState,
        input: &mut KeyedInputNode<KeyboardKey, NoState>,
        dst: &mut SoundChunk,
        ctx: Context,
    ) {
        dst.silence();
        let mut scratch_buffer = SoundChunk::new();
        for kd in input.data_mut() {
            let prev_id = kd.key().previous_id.load(Ordering::SeqCst);
            let curr_id = kd.key().current_id.load(Ordering::SeqCst);
            if curr_id == -1 {
                continue;
            }
            if prev_id == -1 {
                // TODO: gather fine timing data and apply it here
                kd.flag_for_reset();
            }
            kd.step(state, dst, &ctx);
            // TODO: create FMA functions
            numeric::mul_scalar_inplace(&mut scratch_buffer.l, 0.1);
            numeric::mul_scalar_inplace(&mut scratch_buffer.r, 0.1);
            numeric::add_inplace(&mut dst.l, &scratch_buffer.l);
            numeric::add_inplace(&mut dst.r, &scratch_buffer.r);

            kd.key().previous_id.store(curr_id, Ordering::SeqCst);
        }
    }
}

impl WithObjectType for Keyboard {
    const TYPE: ObjectType = ObjectType::new("keyboard");
}
