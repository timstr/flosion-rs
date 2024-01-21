use serialization::{Deserializer, Serializable, Serializer};

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

    pub fn chunks(&self) -> &Vec<SoundChunk> {
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
        ch.l[0] = l;
        ch.r[0] = r;
        self.sample_len += 1;
    }
}

impl Serializable for SoundBuffer {
    fn serialize(&self, serializer: &mut Serializer) {
        serializer.array_iter_f32(self.samples().flatten());
    }

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, ()> {
        let n_samples_x2 = deserializer.peek_length()?;
        if n_samples_x2 % 2 != 0 {
            return Err(());
        }
        let n_samples = n_samples_x2 / 2;
        let mut sample_iter = deserializer.array_iter_f32()?;
        let mut ch = SoundChunk::new();
        let mut i_ch: usize = 0;
        let mut chunks = Vec::<SoundChunk>::with_capacity(n_samples / CHUNK_SIZE);
        while let Some(l) = sample_iter.next() {
            let r = sample_iter.next().unwrap();
            if i_ch == CHUNK_SIZE {
                i_ch = 0;
                chunks.push(ch.clone());
                ch.silence();
            }
            ch.l[i_ch] = l;
            ch.r[i_ch] = r;
        }
        Ok(SoundBuffer {
            chunks,
            sample_len: n_samples,
        })
    }
}
