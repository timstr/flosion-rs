use crate::core::{
    graphobject::{ObjectType, TypedGraphObject},
    key::Key,
    soundinput::{InputOptions, KeyedSoundInputHandle},
    soundprocessor::StaticSoundProcessor,
    soundprocessortools::SoundProcessorTools,
    soundstate::SoundState, soundchunk::SoundChunk, context::ProcessorContext,
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

const MAX_KEYS: usize = 4;

pub struct Keyboard {
    pub input: KeyedSoundInputHandle<KeyboardKey, KeyboardKeyState>,
    active_keys:
}


pub struct KeyboardState {
    // TODO: list of active keys
}

impl Default for KeyboardState {
    fn default() -> KeyboardState {
        KeyboardState {}
    }
}

impl KeyboardState {
    pub fn press_key(&self, key: ???){
        todo!()
    }

    pub fn release_key(&self, key: ???){
        todo!()
    }
}

impl SoundState for KeyboardState {
    fn reset(&mut self) {}
}

impl StaticSoundProcessor for Keyboard {
    type StateType = KeyboardState;

    fn new(tools: &mut SoundProcessorTools<'_, KeyboardState>) -> Keyboard {
        Keyboard {
            input: tools
                .add_keyed_sound_input(InputOptions {
                    interruptible: true,
                    realtime: true,
                })
                .0,
        }
    }

    fn process_audio(
        &self,
        dst: &mut SoundChunk,
        context: ProcessorContext<'_, KeyboardState>,
    ) {
        todo!()
        // TODO:
    }

    fn produces_output(&self) -> bool {
        true
    }
}

impl TypedGraphObject for Keyboard {
    const TYPE: ObjectType = ObjectType::new("keyboard");
}
