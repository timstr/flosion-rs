use super::{numberinput::NumberInputId, numbersource::NumberSourceId};

#[derive(Debug, Clone, Eq, PartialEq)]
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
