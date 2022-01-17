use super::{
    numberinput::NumberInputId, numbersource::NumberSourceId, soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
};

#[derive(Debug)]
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

#[derive(Debug)]
pub struct NumberPath {
    pub connections: Vec<(NumberSourceId, NumberInputId)>,
}

impl NumberPath {
    pub fn new(connections: Vec<(NumberSourceId, NumberInputId)>) -> NumberPath {
        NumberPath { connections }
    }

    pub fn contains_source(&self, source_id: NumberSourceId) -> bool {
        return self
            .connections
            .iter()
            .find(|(nsid, _)| *nsid == source_id)
            .is_some();
    }

    pub fn contains_input(&self, input_id: NumberInputId) -> bool {
        return self
            .connections
            .iter()
            .find(|(_, niid)| *niid == input_id)
            .is_some();
    }

    pub fn trim_until_input(&self, input_id: NumberInputId) -> NumberPath {
        let idx = self
            .connections
            .iter()
            .position(|(_, siid)| *siid == input_id)
            .unwrap();
        let p: Vec<_> = self.connections[idx..].iter().cloned().collect();
        NumberPath { connections: p }
    }

    pub fn push(&mut self, source_id: NumberSourceId, input_id: NumberInputId) {
        self.connections.push((source_id, input_id));
    }

    pub fn pop(&mut self) -> Option<(NumberSourceId, NumberInputId)> {
        self.connections.pop()
    }
}

impl Clone for NumberPath {
    fn clone(&self) -> NumberPath {
        NumberPath {
            connections: self.connections.clone(),
        }
    }
}
