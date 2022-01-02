use super::{
    key::Key,
    soundchunk::SoundChunk,
    soundinput::{KeyedSoundInput, SingleSoundInput, SoundInputId},
    soundprocessor::SoundProcessorId,
    soundstate::{EmptyState, SoundState},
};

pub struct Context<'a> {
    output_buffer: Option<&'a mut SoundChunk>,
    input_buffers: Vec<(SoundInputId, &'a SoundChunk)>,
    processor_id: SoundProcessorId,
    state_index: usize,
}

impl<'a> Context<'a> {
    pub(super) fn new(
        output_buffer: Option<&'a mut SoundChunk>,
        input_buffers: Vec<(SoundInputId, &'a SoundChunk)>,
        processor_id: SoundProcessorId,
        state_index: usize,
    ) -> Context<'a> {
        Context {
            output_buffer,
            input_buffers,
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

    pub fn input_buffer(&'a mut self, input_id: SoundInputId) -> &'a SoundChunk {
        // TODO: if the input buffer is not yet filled, call on the sound graph to fill it now
        match self
            .input_buffers
            .iter_mut()
            .find(|(id, _)| *id == input_id)
        {
            Some((_, buffer)) => *buffer,
            None => panic!(),
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
