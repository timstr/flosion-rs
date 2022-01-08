use std::collections::HashMap;

use super::{
    key::Key,
    soundchunk::SoundChunk,
    soundengine::SoundProcessorData,
    soundinput::{KeyedSoundInput, SingleSoundInput, SingleSoundInputHandle},
    soundprocessor::SoundProcessorId,
    soundstate::{EmptyState, SoundState},
};

pub struct Context<'a> {
    output_buffer: Option<&'a mut SoundChunk>,
    processor_data: &'a HashMap<SoundProcessorId, SoundProcessorData>,
    processor_id: SoundProcessorId,
    state_index: usize,
}

impl<'a> Context<'a> {
    pub(super) fn new(
        output_buffer: Option<&'a mut SoundChunk>,
        processor_data: &'a HashMap<SoundProcessorId, SoundProcessorData>,
        processor_id: SoundProcessorId,
        state_index: usize,
    ) -> Context<'a> {
        Context {
            output_buffer,
            processor_data,
            processor_id,
            state_index,
        }
    }

    pub fn has_output(&self) -> bool {
        match self.output_buffer {
            Some(_) => true,
            None => false,
        }
    }

    pub fn output_buffer(&mut self) -> &mut SoundChunk {
        self.output_buffer.as_mut().unwrap()
    }

    pub fn step_single_input(&mut self, handle: &SingleSoundInputHandle, dst: &mut SoundChunk) {
        let pd = self.processor_data.get(&self.processor_id).unwrap();
        let input = pd.inputs().iter().find(|i| i.id() == handle.id()).unwrap();
        assert!(input.input().num_keys() == 1);
        if let Some(target) = input.target() {
            let other_pd = self.processor_data.get(&target).unwrap();
            // TODO: ???
            assert!(!other_pd.sound_processor().is_static());
            let mut new_ctx =
                Context::new(Some(dst), self.processor_data, target, self.state_index);
            other_pd.sound_processor().process_audio(&mut new_ctx);
        } else {
            dst.silence();
        }
    }

    pub fn single_input_state(&'a self, _input: &SingleSoundInput) -> &mut EmptyState {
        // TODO: assert that the input belongs to the sound processor
        panic!()
    }

    pub fn keyed_input_state<K: Key, T: SoundState>(
        &'a self,
        _input: &KeyedSoundInput<K, T>,
        _key: &K,
    ) -> &mut T {
        // TODO: assert that the input belongs to the sound processor
        panic!()
    }

    pub fn state_index(&self) -> usize {
        self.state_index
    }
}
