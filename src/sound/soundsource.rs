#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Hash)]
pub struct SoundSourceId {
    id: usize,
}

pub struct SoundSourceBaseData {
    sound_inputs: Vec<SoundInputRef>,
    id: SoundSourceId,
}

pub struct SoundSourceBase {
    data: Rc<RefCell<SoundSourceBaseData>>,
}
pub struct SoundSourceBaseRef {
    data: Rc<RefCell<SoundSourceBaseData>>,
}

pub trait SoundSource {
    fn get_next_chunk(&self, context: &SoundContext, chunk: &mut SoundChunk);
    fn base(&self) -> SoundSourceBase;
}
