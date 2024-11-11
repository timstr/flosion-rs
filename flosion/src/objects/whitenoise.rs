use flosion_macros::ProcessorComponents;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};
use rand::prelude::*;

use crate::{
    core::{
        objecttype::{ObjectType, WithObjectType},
        sound::{
            context::AudioContext,
            soundprocessor::{SoundProcessor, StreamStatus},
        },
        soundchunk::SoundChunk,
        stashing::{StashingContext, UnstashingContext},
    },
    ui_core::arguments::ParsedArguments,
};

#[derive(ProcessorComponents)]
pub struct WhiteNoise {}

impl SoundProcessor for WhiteNoise {
    fn new(_args: &ParsedArguments) -> WhiteNoise {
        WhiteNoise {}
    }

    fn is_static(&self) -> bool {
        false
    }

    fn process_audio(
        _whitenoise: &mut CompiledWhiteNoise,
        dst: &mut SoundChunk,
        _context: &mut AudioContext,
    ) -> StreamStatus {
        for s in dst.l.iter_mut() {
            let r: f32 = thread_rng().gen();
            *s = 0.2 * r - 0.1;
        }
        for s in dst.r.iter_mut() {
            let r: f32 = thread_rng().gen();
            *s = 0.2 * r - 0.1;
        }
        StreamStatus::Playing
    }
}

impl WithObjectType for WhiteNoise {
    const TYPE: ObjectType = ObjectType::new("whitenoise");
}

impl Stashable<StashingContext> for WhiteNoise {
    fn stash(&self, _stasher: &mut Stasher<StashingContext>) {}
}

impl<'a> UnstashableInplace<UnstashingContext<'a>> for WhiteNoise {
    fn unstash_inplace(
        &mut self,
        _unstasher: &mut InplaceUnstasher<UnstashingContext>,
    ) -> Result<(), UnstashError> {
        Ok(())
    }
}
