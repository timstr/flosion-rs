use super::soundchunk::{SoundChunk, CHUNK_SIZE};

pub struct SoundBuffer {
    chunks: Vec<SoundChunk>,
    sample_len: usize, // to account for unused portion of last chunk
}

impl SoundBuffer {
    pub fn new_empty() -> SoundBuffer {
        SoundBuffer {
            chunks: Vec::new(),
            sample_len: 0,
        }
    }

    pub fn new(chunks: Vec<SoundChunk>, sample_len: usize) -> SoundBuffer {
        debug_assert!((|| {
            let n_chunks = chunks.len();
            sample_len >= (n_chunks * CHUNK_SIZE) && sample_len < ((n_chunks + 1) * CHUNK_SIZE)
        })());
        SoundBuffer { chunks, sample_len }
    }

    pub fn chunks(&self) -> &Vec<SoundChunk> {
        &self.chunks
    }

    pub fn sample_len(&self) -> usize {
        self.sample_len
    }

    // TODO:
    // - push_sample
    // - push_chunk
    // - push_chunk_partial
    // - extend from other buffer
}
