use std::{ops::Deref, sync::Arc};

use flosion_macros::ProcessorComponent;
use hashstash::{
    HashCache, InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace,
};
use parking_lot::Mutex;

use crate::{
    core::{
        audiofileio::load_audio_file,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            context::AudioContext,
            soundprocessor::{
                ProcessorState, SoundProcessor, StartOver, StateMarker, StreamStatus,
            },
        },
        soundbuffer::SoundBuffer,
        soundchunk::{SoundChunk, CHUNK_SIZE},
        stashing::{StashingContext, UnstashingContext},
    },
    ui_core::arguments::{FilePathArgument, ParsedArguments},
};

#[derive(ProcessorComponent)]
pub struct AudioClip {
    #[not_a_component]
    data: Arc<Mutex<HashCache<SoundBuffer>>>,

    #[state]
    state: StateMarker<AudioClipState>,
}

impl AudioClip {
    pub fn set_data(&self, data: SoundBuffer) {
        **self.data.lock() = data;
    }

    pub fn get_data<'a>(&'a self) -> impl 'a + Deref<Target = HashCache<SoundBuffer>> {
        self.data.lock()
    }
}

impl AudioClip {
    pub const ARG_PATH: FilePathArgument = FilePathArgument("path");
}

pub struct AudioClipState {
    // TODO: make this nicer
    data: Arc<Mutex<HashCache<SoundBuffer>>>,
    playhead: usize,
}

impl ProcessorState for AudioClipState {
    type Processor = AudioClip;

    fn new(processor: &AudioClip) -> Self {
        AudioClipState {
            data: Arc::clone(&processor.data),
            playhead: 0,
        }
    }
}

impl StartOver for AudioClipState {
    fn start_over(&mut self) {
        self.playhead = 0;
    }
}

impl SoundProcessor for AudioClip {
    fn new(args: &ParsedArguments) -> AudioClip {
        let buffer = if let Some(path) = args.get(&Self::ARG_PATH) {
            if let Ok(b) = load_audio_file(&path) {
                b
            } else {
                println!("Failed to load audio file from \"{}\"", path.display());
                SoundBuffer::new_empty()
            }
        } else {
            SoundBuffer::new_empty()
        };
        AudioClip {
            data: Arc::new(Mutex::new(HashCache::new(buffer))),
            state: StateMarker::new(),
        }
    }

    fn is_static(&self) -> bool {
        false
    }

    fn process_audio(
        audioclip: &mut Self::CompiledType<'_>,
        dst: &mut SoundChunk,
        _context: &mut AudioContext,
    ) -> StreamStatus {
        // TODO: avoid locking here? Maybe use ArcSwap
        let data = audioclip.state.data.lock();
        if data.sample_len() == 0 {
            dst.silence();
            return StreamStatus::Done;
        }
        if audioclip.state.playhead >= data.sample_len() {
            audioclip.state.playhead = 0;
        }
        for i in 0..CHUNK_SIZE {
            // TODO: don't repeat this every sample
            let ci = audioclip.state.playhead / CHUNK_SIZE;
            let si = audioclip.state.playhead % CHUNK_SIZE;
            let c = &data.chunks()[ci];
            audioclip.state.playhead += 1;
            if audioclip.state.playhead >= data.sample_len() {
                // TODO: add an option to enable/disable looping
                audioclip.state.playhead = 0;
            }
            debug_assert!(audioclip.state.playhead < data.sample_len());
            dst.l[i] = c.l[si];
            dst.r[i] = c.r[si];
        }
        StreamStatus::Playing
    }
}

impl WithObjectType for AudioClip {
    const TYPE: ObjectType = ObjectType::new("audioclip");
}

impl Stashable<StashingContext> for AudioClip {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        let buffer = self.data.lock();
        let buffer: &HashCache<SoundBuffer> = &buffer;
        stasher.object(buffer);
    }
}

impl<'a> UnstashableInplace<UnstashingContext<'a>> for AudioClip {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext>,
    ) -> Result<(), UnstashError> {
        let mut buffer = self.data.lock();
        let buffer: &mut HashCache<SoundBuffer> = &mut buffer;
        // TODO: make this better
        println!("TODO: stop deserializing the entire audio buffer");
        unstasher.object_replace(buffer)
    }
}

#[cfg(test)]
mod test {
    use hashstash::test_stash_roundtrip_inplace;

    use crate::{
        core::{
            expression::expressionobject::ExpressionObjectFactory,
            sound::{soundobject::SoundObjectFactory, soundprocessor::SoundProcessor},
            soundbuffer::SoundBuffer,
            stashing::{StashingContext, UnstashingContext},
        },
        ui_core::arguments::ParsedArguments,
    };

    use super::AudioClip;

    #[test]
    fn test_stash() {
        let obj_fac = SoundObjectFactory::new_empty();
        let expr_fac = ExpressionObjectFactory::new_empty();

        test_stash_roundtrip_inplace(
            || AudioClip::new(&ParsedArguments::new_empty()),
            |audioclip| {
                let mut sb = SoundBuffer::new_empty();
                sb.push_sample(0.0, 0.0);
                sb.push_sample(1.0, 1.0);
                sb.push_sample(0.0, 0.0);
                sb.push_sample(-1.0, -1.0);
                audioclip.set_data(sb);
            },
            StashingContext::new_stashing_normally(),
            UnstashingContext::new(&obj_fac, &expr_fac),
        )
        .unwrap();
    }
}
