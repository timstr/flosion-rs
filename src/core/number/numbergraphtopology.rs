use std::hash::Hasher;

use crate::core::{
    revision::revision::{Revision, RevisionNumber, Versioned, VersionedHashMap},
    uniqueid::UniqueId,
};

use super::{
    numbergraph::{NumberGraphInputId, NumberGraphOutputId},
    numbergraphdata::{
        NumberDestination, NumberGraphOutputData, NumberInputData, NumberSourceData, NumberTarget,
    },
    numbergrapherror::NumberError,
    numberinput::NumberInputId,
    numbersource::NumberSourceId,
};

#[derive(Clone)]
pub(crate) struct NumberGraphTopology {
    number_sources: VersionedHashMap<NumberSourceId, NumberSourceData>,
    number_inputs: VersionedHashMap<NumberInputId, NumberInputData>,
    graph_inputs: Vec<NumberGraphInputId>,
    graph_outputs: Vec<NumberGraphOutputData>,
}

impl NumberGraphTopology {
    pub(crate) fn new() -> NumberGraphTopology {
        NumberGraphTopology {
            number_sources: VersionedHashMap::new(),
            number_inputs: VersionedHashMap::new(),
            graph_inputs: Vec::new(),
            graph_outputs: Vec::new(),
        }
    }

    pub(crate) fn number_input(&self, id: NumberInputId) -> Option<&Versioned<NumberInputData>> {
        self.number_inputs.get(&id)
    }

    pub(crate) fn number_source(&self, id: NumberSourceId) -> Option<&Versioned<NumberSourceData>> {
        self.number_sources.get(&id)
    }

    pub(super) fn number_source_mut(
        &mut self,
        id: NumberSourceId,
    ) -> Option<&mut Versioned<NumberSourceData>> {
        self.number_sources.get_mut(&id)
    }

    pub(crate) fn number_inputs(&self) -> &VersionedHashMap<NumberInputId, NumberInputData> {
        &self.number_inputs
    }

    pub(crate) fn number_sources(&self) -> &VersionedHashMap<NumberSourceId, NumberSourceData> {
        &self.number_sources
    }

    pub(crate) fn graph_inputs(&self) -> &[NumberGraphInputId] {
        &self.graph_inputs
    }

    pub(crate) fn graph_output(&self, id: NumberGraphOutputId) -> Option<&NumberGraphOutputData> {
        self.graph_outputs.iter().filter(|x| x.id() == id).next()
    }

    pub(crate) fn graph_outputs(&self) -> &[NumberGraphOutputData] {
        &self.graph_outputs
    }

    pub(crate) fn number_target_destinations<'a>(
        &'a self,
        target: NumberTarget,
    ) -> impl 'a + Iterator<Item = NumberDestination> {
        let matching_number_inputs = self.number_inputs.values().filter_map(move |i| {
            if i.target() == Some(target) {
                Some(NumberDestination::Input(i.id()))
            } else {
                None
            }
        });
        let matching_graph_outputs = self.graph_outputs.iter().filter_map(move |i| {
            if i.target() == Some(target) {
                Some(NumberDestination::GraphOutput(i.id()))
            } else {
                None
            }
        });
        matching_number_inputs.chain(matching_graph_outputs)
    }

    pub fn add_number_input(&mut self, data: NumberInputData) -> Result<(), NumberError> {
        if data.target().is_some() {
            return Err(NumberError::BadInputInit(data.id()));
        }

        if self.number_inputs.contains_key(&data.id()) {
            return Err(NumberError::InputIdTaken(data.id()));
        }

        let owner = data.owner();

        let ns_data = self
            .number_sources
            .get_mut(&owner)
            .ok_or(NumberError::SourceNotFound(owner))?;

        debug_assert!(!ns_data.number_inputs().contains(&data.id()));

        ns_data.number_inputs_mut().push(data.id());

        self.number_inputs.insert(data.id(), data);

        Ok(())
    }

    pub(crate) fn remove_number_input(
        &mut self,
        input_id: NumberInputId,
    ) -> Result<(), NumberError> {
        let ni_data = self
            .number_input(input_id)
            .ok_or(NumberError::InputNotFound(input_id))?;
        if ni_data.target().is_some() {
            return Err(NumberError::BadInputCleanup(input_id));
        }
        let ns_data = self.number_sources.get_mut(&ni_data.owner()).unwrap();
        debug_assert_eq!(
            ns_data
                .number_inputs()
                .iter()
                .filter(|x| **x == input_id)
                .count(),
            1
        );
        ns_data.number_inputs_mut().retain(|x| *x != input_id);
        let prev = self.number_inputs.remove(&input_id);
        debug_assert!(prev.is_some());

        Ok(())
    }

    pub(crate) fn add_number_source(&mut self, data: NumberSourceData) -> Result<(), NumberError> {
        if !data.number_inputs().is_empty() {
            return Err(NumberError::BadSourceInit(data.id()));
        }
        if self.number_sources.contains_key(&data.id()) {
            return Err(NumberError::SourceIdTaken(data.id()));
        }
        self.number_sources.insert(data.id(), data);

        Ok(())
    }

    pub(crate) fn remove_number_source(
        &mut self,
        source_id: NumberSourceId,
    ) -> Result<(), NumberError> {
        if !self.number_sources.contains_key(&source_id) {
            return Err(NumberError::SourceNotFound(source_id));
        }

        // Does the number source still own any inputs?
        if self.number_inputs.values().any(|d| d.owner() == source_id) {
            return Err(NumberError::BadSourceCleanup(source_id));
        }
        // Is anything connected to the number source?
        if self.number_target_destinations(source_id.into()).count() > 0 {
            return Err(NumberError::BadSourceCleanup(source_id));
        }

        debug_assert!(self
            .number_sources
            .get(&source_id)
            .unwrap()
            .number_inputs()
            .is_empty());

        self.number_sources.remove(&source_id);

        Ok(())
    }

    pub(crate) fn connect_number_input(
        &mut self,
        input_id: NumberInputId,
        target: NumberTarget,
    ) -> Result<(), NumberError> {
        match target {
            NumberTarget::Source(nsid) => {
                if !self.number_sources.contains_key(&nsid) {
                    return Err(NumberError::SourceNotFound(nsid));
                }
            }
            NumberTarget::GraphInput(giid) => {
                if !self.graph_inputs.contains(&giid) {
                    return Err(NumberError::GraphInputNotFound(giid));
                }
            }
        }
        let data = self
            .number_inputs
            .get_mut(&input_id)
            .ok_or(NumberError::InputNotFound(input_id))?;
        if let Some(current_target) = data.target() {
            return Err(NumberError::InputOccupied {
                input_id,
                current_target,
            });
        }
        data.set_target(Some(target));

        Ok(())
    }

    pub(crate) fn disconnect_number_input(
        &mut self,
        input_id: NumberInputId,
    ) -> Result<(), NumberError> {
        let data = self
            .number_inputs
            .get_mut(&input_id)
            .ok_or(NumberError::InputNotFound(input_id))?;
        if data.target().is_none() {
            return Err(NumberError::InputUnoccupied(input_id));
        }
        data.set_target(None);
        Ok(())
    }

    pub(crate) fn add_graph_input(
        &mut self,
        input_id: NumberGraphInputId,
    ) -> Result<(), NumberError> {
        if self.graph_inputs.contains(&input_id) {
            return Err(NumberError::GraphInputIdTaken(input_id));
        }
        self.graph_inputs.push(input_id);
        Ok(())
    }

    pub(crate) fn remove_graph_input(
        &mut self,
        input_id: NumberGraphInputId,
    ) -> Result<(), NumberError> {
        if self.graph_inputs.iter().filter(|x| **x == input_id).count() != 1 {
            return Err(NumberError::GraphInputNotFound(input_id));
        }
        if self
            .number_inputs
            .values()
            .any(|x| x.target() == Some(NumberTarget::GraphInput(input_id)))
        {
            return Err(NumberError::BadGraphInputCleanup(input_id));
        }
        if self
            .graph_outputs
            .iter()
            .any(|x| x.target() == Some(NumberTarget::GraphInput(input_id)))
        {
            return Err(NumberError::BadGraphInputCleanup(input_id));
        }

        self.graph_inputs.retain(|x| *x != input_id);
        Ok(())
    }

    pub(crate) fn add_graph_output(
        &mut self,
        data: NumberGraphOutputData,
    ) -> Result<(), NumberError> {
        if data.target().is_some() {
            return Err(NumberError::BadGraphOutputInit(data.id()));
        }

        if self
            .graph_outputs
            .iter()
            .filter(|x| x.id() == data.id())
            .count()
            > 0
        {
            return Err(NumberError::GraphOutputIdTaken(data.id()));
        }
        self.graph_outputs.push(data);
        Ok(())
    }

    pub(crate) fn remove_graph_output(
        &mut self,
        output_id: NumberGraphOutputId,
    ) -> Result<(), NumberError> {
        if self
            .graph_outputs
            .iter()
            .filter(|x| x.id() == output_id)
            .count()
            != 1
        {
            return Err(NumberError::BadGraphOutputCleanup(output_id));
        }
        if self
            .graph_outputs
            .iter()
            .filter(|x| x.id() == output_id)
            .next()
            .unwrap()
            .target()
            .is_some()
        {
            return Err(NumberError::BadGraphOutputCleanup(output_id));
        }
        self.graph_outputs.retain(|x| x.id() != output_id);
        Ok(())
    }

    pub(crate) fn connect_graph_output(
        &mut self,
        output_id: NumberGraphOutputId,
        target: NumberTarget,
    ) -> Result<(), NumberError> {
        match target {
            NumberTarget::Source(nsid) => {
                if !self.number_sources.contains_key(&nsid) {
                    return Err(NumberError::SourceNotFound(nsid));
                }
            }
            NumberTarget::GraphInput(giid) => {
                if !self.graph_inputs.contains(&giid) {
                    return Err(NumberError::GraphInputNotFound(giid));
                }
            }
        };
        let data = self
            .graph_outputs
            .iter_mut()
            .filter(|x| x.id() == output_id)
            .next()
            .ok_or(NumberError::GraphOutputNotFound(output_id))?;
        if let Some(current_target) = data.target() {
            return Err(NumberError::GraphOutputOccupied {
                output_id,
                current_target,
            });
        }
        data.set_target(Some(target));
        Ok(())
    }

    pub(crate) fn disconnect_graph_output(
        &mut self,
        output_id: NumberGraphOutputId,
    ) -> Result<(), NumberError> {
        let data = self
            .graph_outputs
            .iter_mut()
            .filter(|x| x.id() == output_id)
            .next()
            .ok_or(NumberError::GraphOutputNotFound(output_id))?;
        if data.target().is_none() {
            return Err(NumberError::GraphOutputUnoccupied(output_id));
        }
        data.set_target(None);
        Ok(())
    }
}

impl Revision for NumberGraphTopology {
    fn get_revision(&self) -> RevisionNumber {
        let mut hasher = seahash::SeaHasher::new();
        hasher.write_u64(self.number_sources.get_revision().value());
        hasher.write_u64(self.number_inputs.get_revision().value());
        hasher.write_usize(self.graph_inputs.len());
        for giid in &self.graph_inputs {
            hasher.write_usize(giid.value());
        }
        hasher.write_usize(self.graph_outputs.len());
        for o in &self.graph_outputs {
            hasher.write_u64(o.get_revision().value());
        }
        RevisionNumber::new(hasher.finish())
    }
}
