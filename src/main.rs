use flosion::make_noise_for_two_seconds;
use flosion::sound::soundchunk::SoundChunk;
use flosion::sound::soundgraph::SoundGraph;

fn main() {
    println!("Hello, world!");

    let sc: SoundChunk = SoundChunk::new();
    let sg: SoundGraph = SoundGraph::new();

    make_noise_for_two_seconds();
}
