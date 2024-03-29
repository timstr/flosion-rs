use super::{soundinput::SoundInputId, soundprocessor::SoundProcessorId};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SoundPath {
    pub connections: Vec<(SoundProcessorId, SoundInputId)>,
}

impl SoundPath {
    pub fn new(connections: Vec<(SoundProcessorId, SoundInputId)>) -> SoundPath {
        SoundPath { connections }
    }

    pub fn contains_processor(&self, processor_id: SoundProcessorId) -> bool {
        return self
            .connections
            .iter()
            .find(|(spid, _)| *spid == processor_id)
            .is_some();
    }

    pub fn contains_input(&self, input_id: SoundInputId) -> bool {
        return self
            .connections
            .iter()
            .find(|(_, siid)| *siid == input_id)
            .is_some();
    }

    pub fn trim_until_input(&self, input_id: SoundInputId) -> SoundPath {
        let idx = self
            .connections
            .iter()
            .position(|(_, siid)| *siid == input_id)
            .unwrap();
        let p: Vec<_> = self.connections[idx..].iter().cloned().collect();
        SoundPath { connections: p }
    }

    pub fn push(&mut self, processor_id: SoundProcessorId, input_id: SoundInputId) {
        self.connections.push((processor_id, input_id));
    }

    pub fn pop(&mut self) -> Option<(SoundProcessorId, SoundInputId)> {
        self.connections.pop()
    }
}
