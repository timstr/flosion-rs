use super::{
    soundgraphdata::{
        SoundInputData, SoundNumberInputData, SoundNumberSourceData, SoundProcessorData,
    },
    soundgrapherror::SoundError,
    soundgraphtopology::SoundGraphTopology,
    soundgraphvalidation::{
        validate_number_connection, validate_sound_connection, validate_sound_disconnection,
    },
    soundinput::SoundInputId,
    soundnumberinput::SoundNumberInputId,
    soundnumbersource::{SoundNumberSourceId, SoundNumberSourceOwner},
    soundprocessor::SoundProcessorId,
};

#[derive(Clone)]
pub(crate) enum SoundGraphEdit {
    AddSoundProcessor(SoundProcessorData),
    RemoveSoundProcessor(SoundProcessorId),
    AddSoundInput(SoundInputData),
    RemoveSoundInput(SoundInputId, SoundProcessorId),
    AddSoundInputKey(SoundInputId, usize),
    RemoveSoundInputKey(SoundInputId, usize),
    ConnectSoundInput(SoundInputId, SoundProcessorId),
    DisconnectSoundInput(SoundInputId),
    AddNumberSource(SoundNumberSourceData),
    RemoveNumberSource(SoundNumberSourceId, SoundNumberSourceOwner),
    AddNumberInput(SoundNumberInputData),
    RemoveNumberInput(SoundNumberInputId, SoundProcessorId),
    ConnectNumberInput(SoundNumberInputId, SoundNumberSourceId),
    DisconnectNumberInput(SoundNumberInputId, SoundNumberSourceId),
}

impl SoundGraphEdit {
    pub(super) fn name(&self) -> &'static str {
        match self {
            SoundGraphEdit::AddSoundProcessor(_) => "AddSoundProcessor",
            SoundGraphEdit::RemoveSoundProcessor(_) => "RemoveSoundProcessor",
            SoundGraphEdit::AddSoundInput(_) => "AddSoundInput",
            SoundGraphEdit::RemoveSoundInput(_, _) => "RemoveSoundInput",
            SoundGraphEdit::AddSoundInputKey(_, _) => "AddSoundInputKey",
            SoundGraphEdit::RemoveSoundInputKey(_, _) => "RemoveSoundInputKey",
            SoundGraphEdit::ConnectSoundInput(_, _) => "ConnectSoundInput",
            SoundGraphEdit::DisconnectSoundInput(_) => "DisconnectSoundInput",
            SoundGraphEdit::AddNumberSource(_) => "AddNumberSource",
            SoundGraphEdit::RemoveNumberSource(_, _) => "RemoveNumberSource",
            SoundGraphEdit::AddNumberInput(_) => "AddNumberInput",
            SoundGraphEdit::RemoveNumberInput(_, _) => "RemoveNumberInput",
            SoundGraphEdit::ConnectNumberInput(_, _) => "ConnectNumberInput",
            SoundGraphEdit::DisconnectNumberInput(_, _) => "DisconnectNumberInput",
        }
    }

    // Ensures that all entities needed by the edit already exist, and that any
    // entities added by the edit do not introduce id collisions.
    pub(super) fn check_preconditions(&self, topo: &SoundGraphTopology) -> Option<SoundError> {
        match self {
            SoundGraphEdit::AddSoundProcessor(data) => {
                // The processor id must not be taken
                if topo.sound_processor(data.id()).is_some() {
                    return Some(SoundError::ProcessorIdTaken(data.id()));
                }

                // the processor must have no sound inputs
                if !data.sound_inputs().is_empty() {
                    return Some(SoundError::BadProcessorInit(data.id()));
                }

                // the processor must have no number sources
                if !data.number_sources().is_empty() {
                    return Some(SoundError::BadProcessorInit(data.id()));
                }

                // the processor must have no number inputs
                if !data.number_inputs().is_empty() {
                    return Some(SoundError::BadProcessorInit(data.id()));
                }
            }
            SoundGraphEdit::RemoveSoundProcessor(spid) => {
                // the processor must exist
                let data = match topo.sound_processor(*spid) {
                    Some(data) => data,
                    None => {
                        return Some(SoundError::ProcessorNotFound(*spid));
                    }
                };

                // it may not be connected to any sound inputs
                for si in topo.sound_inputs().values() {
                    if si.target() == Some(*spid) {
                        return Some(SoundError::BadProcessorCleanup(*spid));
                    }
                }

                // all its sound inputs must be removed
                if !data.sound_inputs().is_empty() {
                    return Some(SoundError::BadProcessorCleanup(*spid));
                }

                // all its number sources must be removed
                if !data.number_sources().is_empty() {
                    return Some(SoundError::BadProcessorCleanup(*spid));
                }

                // all number inputs must be disconnected
                if !data.number_inputs().is_empty() {
                    return Some(SoundError::BadProcessorCleanup(*spid));
                }
            }
            SoundGraphEdit::AddSoundInput(data) => {
                // the input id must not be taken
                if topo.sound_input(data.id()).is_some() {
                    return Some(SoundError::SoundInputIdTaken(data.id()));
                }

                // the owner processor must exist
                if topo.sound_processor(data.owner()).is_none() {
                    return Some(SoundError::BadSoundInputInit(data.id()));
                }

                // the input must be vacant
                if data.target().is_some() {
                    return Some(SoundError::BadSoundInputInit(data.id()));
                }

                // the input must have no number sources
                if !data.number_sources().is_empty() {
                    return Some(SoundError::BadSoundInputInit(data.id()));
                }
            }
            SoundGraphEdit::RemoveSoundInput(siid, owner_spid) => {
                // the sound input must exist
                let data = match topo.sound_input(*siid) {
                    Some(data) => data,
                    None => {
                        return Some(SoundError::SoundInputNotFound(*siid));
                    }
                };

                // the sound input's owner must match and exist
                if *owner_spid != data.owner() || topo.sound_processor(*owner_spid).is_none() {
                    return Some(SoundError::BadSoundInputCleanup(*siid));
                }

                // the sound input must not be connected
                if data.target().is_some() {
                    return Some(SoundError::BadSoundInputCleanup(*siid));
                }

                // the sound input must have no number sources
                if !data.number_sources().is_empty() {
                    return Some(SoundError::BadSoundInputCleanup(*siid));
                }
            }
            SoundGraphEdit::AddSoundInputKey(siid, index) => {
                // the sound input must exist
                let data = match topo.sound_input(*siid) {
                    Some(data) => data,
                    None => {
                        return Some(SoundError::SoundInputNotFound(*siid));
                    }
                };

                // the index must be at most num_keys
                if *index > data.num_keys() {
                    return Some(SoundError::BadSoundInputKeyIndex(*siid, *index));
                }
            }
            SoundGraphEdit::RemoveSoundInputKey(siid, index) => {
                // the sound input must exist
                let data = match topo.sound_input(*siid) {
                    Some(data) => data,
                    None => {
                        return Some(SoundError::SoundInputNotFound(*siid));
                    }
                };

                // the index must be at most num_keys-1
                if *index >= data.num_keys() {
                    return Some(SoundError::BadSoundInputKeyIndex(*siid, *index));
                }
            }
            SoundGraphEdit::ConnectSoundInput(siid, spid) => {
                // the input must exist
                let data = match topo.sound_input(*siid) {
                    Some(data) => data,
                    None => {
                        return Some(SoundError::SoundInputNotFound(*siid));
                    }
                };

                // the processor must exist
                if topo.sound_processor(*spid).is_none() {
                    return Some(SoundError::ProcessorNotFound(*spid));
                }

                // the input must be vacant
                if let Some(target) = data.target() {
                    return Some(
                        SoundError::SoundInputOccupied {
                            input_id: *siid,
                            current_target: target,
                        }
                        .into(),
                    );
                }

                // the connection must be legal
                if let Err(e) = validate_sound_connection(topo, *siid, *spid) {
                    return Some(e);
                }
            }
            SoundGraphEdit::DisconnectSoundInput(siid) => {
                // the sound input must exist
                let data = match topo.sound_input(*siid) {
                    Some(data) => data,
                    None => {
                        return Some(SoundError::SoundInputNotFound(*siid));
                    }
                };

                // the sound input must be occupied
                if data.target().is_none() {
                    return Some(SoundError::SoundInputUnoccupied(*siid));
                }

                // the sound input must be safe to disconnect
                if let Err(e) = validate_sound_disconnection(topo, *siid) {
                    return Some(e);
                }
            }
            SoundGraphEdit::AddNumberSource(data) => {
                // the source's id must not be taken
                if topo.number_source(data.id()).is_some() {
                    return Some(SoundError::NumberSourceIdTaken(data.id()));
                }

                // the source's owner must exist
                match data.owner() {
                    SoundNumberSourceOwner::SoundProcessor(spid) => {
                        if topo.sound_processor(spid).is_none() {
                            return Some(SoundError::BadNumberSourceInit(data.id()));
                        }
                    }
                    SoundNumberSourceOwner::SoundInput(siid) => {
                        if topo.sound_input(siid).is_none() {
                            return Some(SoundError::BadNumberSourceInit(data.id()));
                        }
                    }
                }
            }
            SoundGraphEdit::RemoveNumberSource(nsid, owner_id) => {
                // the source must exist
                let data = match topo.number_source(*nsid) {
                    Some(data) => data,
                    None => return Some(SoundError::NumberSourceNotFound(*nsid).into()),
                };

                // the owner must match and exist
                if *owner_id != data.owner() {
                    return Some(SoundError::BadNumberSourceCleanup(*nsid));
                }

                // the owner must cross-list the number source correctly
                match *owner_id {
                    SoundNumberSourceOwner::SoundProcessor(spid) => {
                        match topo.sound_processor(spid) {
                            Some(sp) => {
                                if !sp.number_sources().contains(&nsid) {
                                    return Some(SoundError::BadNumberSourceCleanup(*nsid));
                                }
                            }
                            None => return Some(SoundError::BadNumberSourceCleanup(*nsid).into()),
                        }
                    }
                    SoundNumberSourceOwner::SoundInput(siid) => match topo.sound_input(siid) {
                        Some(si) => {
                            if !si.number_sources().contains(&nsid) {
                                return Some(SoundError::BadNumberSourceCleanup(*nsid));
                            }
                        }
                        None => return Some(SoundError::BadNumberSourceCleanup(*nsid).into()),
                    },
                }

                // the source must not be connected to any inputs
                for ni in topo.number_inputs().values() {
                    if ni.targets().contains(nsid) {
                        return Some(SoundError::BadNumberSourceCleanup(*nsid));
                    }
                }
            }
            SoundGraphEdit::AddNumberInput(data) => {
                // the number input's id must not be taken
                if topo.number_input(data.id()).is_some() {
                    return Some(SoundError::NumberInputIdTaken(data.id()));
                }

                // the input's owner must exist
                if topo.sound_processor(data.owner()).is_none() {
                    return Some(SoundError::BadNumberInputInit(data.id()));
                }

                // the input must not be connected
                if !data.targets().is_empty() {
                    return Some(SoundError::BadNumberInputInit(data.id()));
                }
            }
            SoundGraphEdit::RemoveNumberInput(niid, owner_id) => {
                // the number input must exist
                let data = match topo.number_input(*niid) {
                    Some(data) => data,
                    None => return Some(SoundError::NumberInputNotFound(*niid).into()),
                };

                // TODO: is owner_id really needed?
                assert_eq!(data.owner(), *owner_id);

                // the owner must match and exist
                match topo.sound_processor(data.owner()) {
                    Some(sp) => {
                        if !sp.number_inputs().contains(niid) {
                            return Some(SoundError::BadNumberInputCleanup(*niid));
                        }
                    }
                    None => return Some(SoundError::BadNumberInputCleanup(*niid).into()),
                }

                // the number input must not be connected
                if !data.targets().is_empty() {
                    return Some(SoundError::BadNumberInputCleanup(*niid));
                }
            }
            SoundGraphEdit::ConnectNumberInput(niid, nsid) => {
                // the number input must exist
                if topo.number_input(*niid).is_none() {
                    return Some(SoundError::NumberInputNotFound(*niid));
                }

                // the number source must exist
                if topo.number_source(*nsid).is_none() {
                    return Some(SoundError::NumberSourceNotFound(*nsid));
                }

                // the number input must be vacant
                if topo.number_input(*niid).unwrap().targets().contains(nsid) {
                    return Some(
                        SoundError::NumberInputAlreadyConnected {
                            input_id: *niid,
                            target: *nsid,
                        }
                        .into(),
                    );
                }

                // the connection must be legal
                if let Err(e) = validate_number_connection(topo, *niid, *nsid) {
                    return Some(e);
                }
            }
            SoundGraphEdit::DisconnectNumberInput(niid, nsid) => {
                // the number input must exist
                let data = match topo.number_input(*niid) {
                    Some(data) => data,
                    None => return Some(SoundError::NumberInputNotFound(*niid).into()),
                };

                // the number input must be occupied
                if !data.targets().contains(nsid) {
                    return Some(SoundError::NumberInputNotConnected {
                        input_id: *niid,
                        target: *nsid,
                    });
                }
            }
        }
        None
    }
}
