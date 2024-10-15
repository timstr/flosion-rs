use hashstash::{Stashable, Stasher, UnstashError, Unstashable, Unstasher};

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

    pub fn new_with_capacity(num_chunks: usize) -> SoundBuffer {
        SoundBuffer {
            chunks: Vec::with_capacity(num_chunks),
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

    pub fn reserve_chunks(&mut self, additional_chunks: usize) {
        self.chunks.reserve(additional_chunks);
    }

    pub fn chunk_capacity(&self) -> usize {
        self.chunks.capacity()
    }

    pub fn chunks(&self) -> &[SoundChunk] {
        &self.chunks
    }

    pub fn sample_len(&self) -> usize {
        self.sample_len
    }

    pub fn samples_l<'a>(&'a self) -> impl 'a + Iterator<Item = f32> {
        self.chunks.iter().map(|c| &c.l[..]).flatten().cloned()
    }

    pub fn samples_r<'a>(&'a self) -> impl 'a + Iterator<Item = f32> {
        self.chunks.iter().map(|c| &c.r[..]).flatten().cloned()
    }

    pub fn samples<'a>(&'a self) -> impl 'a + Iterator<Item = [f32; 2]> {
        self.chunks
            .iter()
            .flat_map(|c| c.l.iter().cloned().zip(c.r.iter().cloned()))
            .map(|(l, r)| [l, r])
    }

    pub fn push_chunk(&mut self, ch: &SoundChunk) {
        let offset = self.sample_len % CHUNK_SIZE;
        let split_ch = CHUNK_SIZE - offset;
        if offset > 0 {
            let dst = self.chunks.last_mut().unwrap();
            slicemath::copy(&ch.l[..split_ch], &mut dst.l[offset..]);
            slicemath::copy(&ch.r[..split_ch], &mut dst.r[offset..]);
            let mut new_ch = SoundChunk::new();
            slicemath::copy(&ch.l[split_ch..], &mut new_ch.l[..offset]);
            slicemath::copy(&ch.r[split_ch..], &mut new_ch.r[..offset]);
            self.chunks.push(new_ch);
        } else {
            self.chunks.push(*ch);
        }
        self.sample_len += CHUNK_SIZE;
    }

    pub fn push_sample(&mut self, l: f32, r: f32) {
        let offset = self.sample_len % CHUNK_SIZE;
        if offset == 0 {
            self.chunks.push(SoundChunk::new());
        }
        let ch = self.chunks.last_mut().unwrap();
        ch.l[offset] = l;
        ch.r[offset] = r;
        self.sample_len += 1;
    }
}

impl Stashable for SoundBuffer {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.array_of_f32_iter(self.samples().flatten());
    }
}

impl Unstashable for SoundBuffer {
    fn unstash(unstasher: &mut Unstasher) -> Result<Self, UnstashError> {
        let mut samples = unstasher.array_of_f32_iter()?;

        let mut buffer = SoundBuffer::new_empty();

        loop {
            let Some(l) = samples.next() else {
                break;
            };
            let Some(r) = samples.next() else {
                panic!("Uh oh");
            };

            buffer.push_sample(l, r);
        }

        Ok(buffer)
    }
}
