use flosion_macros::ProcessorComponents;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};
use rand::prelude::*;

use crate::{
    core::{
        engine::soundgraphcompiler::SoundGraphCompiler,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            context::Context,
            soundprocessor::{
                ProcessorComponent, ProcessorComponentVisitor, ProcessorComponentVisitorMut,
                SoundProcessor, SoundProcessorId, StartOver, StreamStatus,
            },
        },
        soundchunk::SoundChunk,
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
        _context: &mut Context,
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

impl Stashable for WhiteNoise {
    fn stash(&self, _stasher: &mut Stasher) {}
}

impl UnstashableInplace for WhiteNoise {
    fn unstash_inplace(&mut self, _unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        Ok(())
    }
}
