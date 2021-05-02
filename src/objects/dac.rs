use sound::soundgraph::{SoundInput, SoundSource};

struct DAC {
    input: SoundInput,
}

impl SoundSource for DAC {}
