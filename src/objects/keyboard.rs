use std::sync::atomic::{AtomicI16, Ordering};

use atomic_float::AtomicF32;

use crate::core::{
    context::ProcessorContext,
    graphobject::{ObjectType, WithObjectType},
    key::Key,
    numbersource::NumberSourceHandle,
    numeric,
    soundchunk::SoundChunk,
    soundinput::{InputOptions, KeyedSoundInputHandle},
    soundprocessor::StaticSoundProcessor,
    soundprocessortools::SoundProcessorTools,
    soundstate::SoundState,
};

#[derive(Ord, PartialOrd, PartialEq, Eq)]
pub struct KeyboardKey {
    index: u8,
}

impl Key for KeyboardKey {}

pub struct KeyboardKeyState {
    pub frequency: f32,
    pub elapsed_time: f32,
}

impl Default for KeyboardKeyState {
    fn default() -> KeyboardKeyState {
        KeyboardKeyState {
            frequency: 0.0,
            elapsed_time: 0.0,
        }
    }
}

impl SoundState for KeyboardKeyState {
    fn reset(&mut self) {
        self.frequency = 0.0;
        self.elapsed_time = 0.0;
    }
}

const MAX_KEYS: usize = 8;

struct KeyState {
    frequency: AtomicF32,
    id: AtomicI16,
}

impl KeyState {
    fn new() -> KeyState {
        KeyState {
            frequency: AtomicF32::new(f32::NAN),
            id: AtomicI16::new(-1),
        }
    }
}

impl Clone for KeyState {
    fn clone(&self) -> KeyState {
        KeyState {
            frequency: AtomicF32::new(self.frequency.load(Ordering::SeqCst)),
            id: AtomicI16::new(self.id.load(Ordering::SeqCst)),
        }
    }
}

pub struct KeyboardState {
    previous_key_state: [KeyState; MAX_KEYS],
}

impl Default for KeyboardState {
    fn default() -> KeyboardState {
        KeyboardState {
            previous_key_state: [(); MAX_KEYS].map(|_| KeyState::new()),
        }
    }
}

impl SoundState for KeyboardState {
    fn reset(&mut self) {}
}

pub struct Keyboard {
    pub input: KeyedSoundInputHandle<KeyboardKey, KeyboardKeyState>,
    pub key_frequency: NumberSourceHandle,
    key_states: [KeyState; MAX_KEYS],
}

impl Keyboard {
    pub fn press_key(&self, key_id: u16, frequency: f32) {
        for ks in self.key_states.iter() {
            if ks.id.load(Ordering::SeqCst) == (key_id as i16) {
                return;
            }
        }
        for ks in self.key_states.iter() {
            if ks.id.load(Ordering::SeqCst) == -1 {
                ks.id.store(key_id as i16, Ordering::SeqCst);
                ks.frequency.store(frequency, Ordering::SeqCst);
                return;
            }
        }
        println!("Warning: keyboard is out of keys to press");
    }

    pub fn release_key(&self, key_id: u16) {
        for ks in self.key_states.iter() {
            if ks.id.load(Ordering::SeqCst) == (key_id as i16) {
                ks.id.store(-1, Ordering::SeqCst);
                ks.frequency.store(f32::NAN, Ordering::SeqCst);
                return;
            }
        }
        println!("Warning: keyboard attempted to release a key which was not held");
    }
}

impl StaticSoundProcessor for Keyboard {
    type StateType = KeyboardState;

    fn new(tools: &mut SoundProcessorTools<'_, KeyboardState>) -> Keyboard {
        let mut input = tools
            .add_keyed_sound_input(InputOptions {
                interruptible: true,
                realtime: true,
            })
            .0;
        let key_frequency = tools
            .add_keyed_input_number_source(&input, |dst: &mut [f32], state: &KeyboardKeyState| {
                numeric::fill(dst, state.frequency)
            })
            .0;
        for i in 0..MAX_KEYS {
            input.add_key(KeyboardKey { index: i as u8 }, tools);
        }
        Keyboard {
            input,
            key_frequency,
            key_states: [(); MAX_KEYS].map(|_| KeyState::new()),
        }
    }

    fn process_audio(
        &self,
        dst: &mut SoundChunk,
        mut context: ProcessorContext<'_, KeyboardState>,
    ) {
        dst.silence();
        let mut scratch_buffer = SoundChunk::new();
        let mut prev_state: [KeyState; MAX_KEYS] = context.read_state().previous_key_state.clone();
        let mut new_states = prev_state.clone();
        for i in 0..MAX_KEYS {
            let curr_id = self.key_states[i].id.load(Ordering::SeqCst);
            *new_states[i].id.get_mut() = curr_id;
            if curr_id == -1 {
                continue;
            }
            if *prev_state[i].id.get_mut() == -1 {
                context.reset_keyed_input(&self.input, i);
                context.keyed_input_state(&self.input, i).write().frequency =
                    self.key_states[i].frequency.load(Ordering::SeqCst);
            }
            context.step_keyed_input(&self.input, i, &mut scratch_buffer);
            // TODO: create FMA functions
            numeric::mul_scalar_inplace(&mut scratch_buffer.l, 0.1);
            numeric::mul_scalar_inplace(&mut scratch_buffer.r, 0.1);
            numeric::add_inplace(&mut dst.l, &scratch_buffer.l);
            numeric::add_inplace(&mut dst.r, &scratch_buffer.r);
        }
        // TODO: write to state's previous_key_state
        {
            let mut state = context.write_state();
            for (ks_next, ks_curr) in new_states
                .iter_mut()
                .zip(state.previous_key_state.iter_mut())
            {
                *ks_curr.id.get_mut() = *ks_next.id.get_mut();
            }
        }
    }

    fn produces_output(&self) -> bool {
        true
    }
}

impl WithObjectType for Keyboard {
    const TYPE: ObjectType = ObjectType::new("keyboard");
}
