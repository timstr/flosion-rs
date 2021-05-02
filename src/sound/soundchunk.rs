use crate::sound::sample::Sample;

pub const CHUNK_SIZE: usize = 1024;

pub struct SoundChunk {
    data: [Sample; CHUNK_SIZE],
}

impl SoundChunk {
    pub fn new() -> SoundChunk {
        SoundChunk {
            data: [Sample::default(); CHUNK_SIZE],
        }
    }

    pub fn silence(&mut self) {
        for s in self.data.iter_mut() {
            s.silence();
        }
    }
}
