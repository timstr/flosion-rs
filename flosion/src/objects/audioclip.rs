use std::{ops::Deref, sync::Arc};

use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};
use parking_lot::RwLock;

use crate::{
    core::{
        audiofileio::load_audio_file,
        engine::soundgraphcompiler::SoundGraphCompiler,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            context::Context,
            soundprocessor::{
                CompiledSoundProcessor, ProcessorComponent, ProcessorComponentVisitor,
                ProcessorComponentVisitorMut, SoundProcessor, SoundProcessorId, StartOver,
                StreamStatus,
            },
            state::State,
        },
        soundbuffer::SoundBuffer,
        soundchunk::{SoundChunk, CHUNK_SIZE},
    },
    ui_core::arguments::{FilePathArgument, ParsedArguments},
};

pub struct AudioClip {
    data: Arc<RwLock<SoundBuffer>>,
}

impl AudioClip {
    pub fn set_data(&self, data: SoundBuffer) {
        *self.data.write() = data;
    }

    pub fn get_data<'a>(&'a self) -> impl 'a + Deref<Target = SoundBuffer> {
        self.data.read()
    }
}

pub struct AudioClipState {
    data: Arc<RwLock<SoundBuffer>>,
    playhead: usize,
}

impl AudioClip {
    pub const ARG_PATH: FilePathArgument = FilePathArgument("path");
}

impl State for AudioClipState {
    fn start_over(&mut self) {
        self.playhead = 0;
    }
}

pub struct CompiledAudioclip {
    state: AudioClipState,
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
            data: Arc::new(RwLock::new(buffer)),
        }
    }

    fn is_static(&self) -> bool {
        false
    }
}

impl ProcessorComponent for AudioClip {
    type CompiledType<'ctx> = CompiledAudioclip;

    fn visit<'a>(&self, _visitor: &'a mut dyn ProcessorComponentVisitor) {}

    fn visit_mut<'a>(&mut self, _visitor: &'a mut dyn ProcessorComponentVisitorMut) {}

    fn compile<'ctx>(
        &self,
        _id: SoundProcessorId,
        _compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx> {
        CompiledAudioclip {
            state: AudioClipState {
                data: Arc::clone(&self.data),
                playhead: 0,
            },
        }
    }
}

impl StartOver for CompiledAudioclip {
    fn start_over(&mut self) {
        self.state.start_over();
    }
}

impl<'ctx> CompiledSoundProcessor<'ctx> for CompiledAudioclip {
    fn process_audio(&mut self, dst: &mut SoundChunk, _context: &mut Context) -> StreamStatus {
        // TODO: avoid locking here? Maybe use ArcSwap
        let data = self.state.data.read();
        if data.sample_len() == 0 {
            dst.silence();
            return StreamStatus::Done;
        }
        if self.state.playhead >= data.sample_len() {
            self.state.playhead = 0;
        }
        for i in 0..CHUNK_SIZE {
            // TODO: don't repeat this every sample
            let ci = self.state.playhead / CHUNK_SIZE;
            let si = self.state.playhead % CHUNK_SIZE;
            let c = &data.chunks()[ci];
            self.state.playhead += 1;
            if self.state.playhead >= data.sample_len() {
                // TODO: add an option to enable/disable looping
                self.state.playhead = 0;
            }
            debug_assert!(self.state.playhead < data.sample_len());
            dst.l[i] = c.l[si];
            dst.r[i] = c.r[si];
        }
        StreamStatus::Playing
    }
}

impl WithObjectType for AudioClip {
    const TYPE: ObjectType = ObjectType::new("audioclip");
}

impl Stashable for AudioClip {
    fn stash(&self, stasher: &mut Stasher) {
        todo!()
    }
}

impl UnstashableInplace for AudioClip {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        todo!()
    }
}
