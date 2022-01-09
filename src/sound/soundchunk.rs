pub const CHUNK_SIZE: usize = 1024;

// TODO: consider making each of these a reference?
pub struct SoundChunk {
    pub l: [f32; CHUNK_SIZE],
    pub r: [f32; CHUNK_SIZE],
}

impl SoundChunk {
    pub fn new() -> SoundChunk {
        SoundChunk {
            l: [0.0; CHUNK_SIZE],
            r: [0.0; CHUNK_SIZE],
        }
    }

    pub fn silence(&mut self) {
        for s in self.l.iter_mut() {
            *s = 0.0;
        }
        for s in self.r.iter_mut() {
            *s = 0.0;
        }
    }
}
