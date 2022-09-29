pub const CHUNK_SIZE: usize = 1024;

#[derive(Clone, Copy)]
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

    pub fn copy_from(&mut self, other: &SoundChunk) {
        self.l = other.l;
        self.r = other.r;
    }

    pub fn samples<'a>(&'a self) -> impl 'a + Iterator<Item = (f32, f32)> {
        self.l.iter().cloned().zip(self.r.iter().cloned())
    }

    pub fn samples_mut<'a>(&'a mut self) -> impl 'a + Iterator<Item = (&mut f32, &mut f32)> {
        self.l.iter_mut().zip(self.r.iter_mut())
    }
}
