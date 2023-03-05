use super::{
    numberinput::{NumberInputId, NumberInputOwner},
    numbersource::{NumberSourceId, NumberSourceOwner},
    soundgraphdata::{NumberInputData, NumberSourceData, SoundInputData, SoundProcessorData},
    soundgrapherror::{NumberError, SoundError, SoundGraphError},
    soundgraphtopology::SoundGraphTopology,
    soundgraphvalidation::{
        validate_number_connection, validate_sound_connection, validate_sound_disconnection,
    },
    soundinput::SoundInputId,
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
    AddNumberSource(NumberSourceData),
    RemoveNumberSource(NumberSourceId, NumberSourceOwner),
    AddNumberInput(NumberInputData),
    RemoveNumberInput(NumberInputId, NumberInputOwner),
    ConnectNumberInput(NumberInputId, NumberSourceId),
    DisconnectNumberInput(NumberInputId),
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
            SoundGraphEdit::DisconnectNumberInput(_) => "DisconnectNumberInput",
        }
    }

    // Ensures that all entities needed by the edit already exist, and that any
    // entities added by the edit do not introduce id collisions.
    pub(super) fn check_preconditions(&self, topo: &SoundGraphTopology) -> Option<SoundGraphError> {
        match self {
            SoundGraphEdit::AddSoundProcessor(data) => {
                // The processor id must not be taken
                if topo.sound_processor(data.id()).is_some() {
                    return Some(SoundError::ProcessorIdTaken(data.id()).into());
                }

                // the processor must have no sound inputs
                if !data.sound_inputs().is_empty() {
                    return Some(SoundError::BadProcessorInit(data.id()).into());
                }

                // the processor must have no number sources
                if !data.number_sources().is_empty() {
                    return Some(SoundError::BadProcessorInit(data.id()).into());
                }

                // the processor must have no number inputs
                if !data.number_inputs().is_empty() {
                    return Some(SoundError::BadProcessorInit(data.id()).into());
                }
            }
            SoundGraphEdit::RemoveSoundProcessor(spid) => {
                // the processor must exist
                let data = match topo.sound_processor(*spid) {
                    Some(data) => data,
                    None => {
                        return Some(SoundError::ProcessorNotFound(*spid).into());
                    }
                };

                // it may not be connected to any sound inputs
                for si in topo.sound_inputs().values() {
                    if si.target() == Some(*spid) {
                        return Some(SoundError::BadProcessorCleanup(*spid).into());
                    }
                }

                // all its sound inputs must be removed
                if !data.sound_inputs().is_empty() {
                    return Some(SoundError::BadProcessorCleanup(*spid).into());
                }

                // all its number sources must be removed
                if !data.number_sources().is_empty() {
                    return Some(SoundError::BadProcessorCleanup(*spid).into());
                }

                // all number inputs must be disconnected
                if !data.number_inputs().is_empty() {
                    return Some(SoundError::BadProcessorCleanup(*spid).into());
                }
            }
            SoundGraphEdit::AddSoundInput(data) => {
                // the input id must not be taken
                if topo.sound_input(data.id()).is_some() {
                    return Some(SoundError::InputIdTaken(data.id()).into());
                }

                // the owner processor must exist
                if topo.sound_processor(data.owner()).is_none() {
                    return Some(SoundError::BadInputInit(data.id()).into());
                }

                // the input must be vacant
                if data.target().is_some() {
                    return Some(SoundError::BadInputInit(data.id()).into());
                }

                // the input must have no number sources
                if !data.number_sources().is_empty() {
                    return Some(SoundError::BadInputInit(data.id()).into());
                }
            }
            SoundGraphEdit::RemoveSoundInput(siid, owner_spid) => {
                // the sound input must exist
                let data = match topo.sound_input(*siid) {
                    Some(data) => data,
                    None => {
                        return Some(SoundError::InputNotFound(*siid).into());
                    }
                };

                // the sound input's owner must match and exist
                if *owner_spid != data.owner() || topo.sound_processor(*owner_spid).is_none() {
                    return Some(SoundError::BadInputCleanup(*siid).into());
                }

                // the sound input must not be connected
                if data.target().is_some() {
                    return Some(SoundError::BadInputCleanup(*siid).into());
                }

                // the sound input must have no number sources
                if !data.number_sources().is_empty() {
                    return Some(SoundError::BadInputCleanup(*siid).into());
                }
            }
            SoundGraphEdit::AddSoundInputKey(siid, index) => {
                // the sound input must exist
                let data = match topo.sound_input(*siid) {
                    Some(data) => data,
                    None => {
                        return Some(SoundError::InputNotFound(*siid).into());
                    }
                };

                // the index must be at most num_keys
                if *index > data.num_keys() {
                    return Some(SoundError::BadInputKeyIndex(*siid, *index).into());
                }
            }
            SoundGraphEdit::RemoveSoundInputKey(siid, index) => {
                // the sound input must exist
                let data = match topo.sound_input(*siid) {
                    Some(data) => data,
                    None => {
                        return Some(SoundError::InputNotFound(*siid).into());
                    }
                };

                // the index must be at most num_keys-1
                if *index >= data.num_keys() {
                    return Some(SoundError::BadInputKeyIndex(*siid, *index).into());
                }
            }
            SoundGraphEdit::ConnectSoundInput(siid, spid) => {
                // the input must exist
                let data = match topo.sound_input(*siid) {
                    Some(data) => data,
                    None => {
                        return Some(SoundError::InputNotFound(*siid).into());
                    }
                };

                // the processor must exist
                if topo.sound_processor(*spid).is_none() {
                    return Some(SoundError::ProcessorNotFound(*spid).into());
                }

                // the input must be vacant
                if let Some(target) = data.target() {
                    return Some(
                        SoundError::InputOccupied {
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
                        return Some(SoundError::InputNotFound(*siid).into());
                    }
                };

                // the sound input must be occupied
                if data.target().is_none() {
                    return Some(SoundError::InputUnoccupied(*siid).into());
                }

                // the sound input must be safe to disconnect
                if let Err(e) = validate_sound_disconnection(topo, *siid) {
                    return Some(e);
                }
            }
            SoundGraphEdit::AddNumberSource(data) => {
                // the source's id must not be taken
                if topo.number_source(data.id()).is_some() {
                    return Some(NumberError::SourceIdTaken(data.id()).into());
                }

                // the source's owner must exist
                match data.owner() {
                    NumberSourceOwner::Nothing => (),
                    NumberSourceOwner::SoundProcessor(spid) => {
                        if topo.sound_processor(spid).is_none() {
                            return Some(NumberError::BadSourceInit(data.id()).into());
                        }
                    }
                    NumberSourceOwner::SoundInput(siid) => {
                        if topo.sound_input(siid).is_none() {
                            return Some(NumberError::BadSourceInit(data.id()).into());
                        }
                    }
                }

                // the source must have no inputs
                if !data.inputs().is_empty() {
                    return Some(NumberError::BadSourceInit(data.id()).into());
                }
            }
            SoundGraphEdit::RemoveNumberSource(nsid, owner_id) => {
                // the source must exist
                let data = match topo.number_source(*nsid) {
                    Some(data) => data,
                    None => return Some(NumberError::SourceNotFound(*nsid).into()),
                };

                // the owner must match and exist
                if *owner_id != data.owner() {
                    return Some(NumberError::BadSourceCleanup(*nsid).into());
                }

                // the owner must cross-list the number source correctly
                match *owner_id {
                    NumberSourceOwner::Nothing => (),
                    NumberSourceOwner::SoundProcessor(spid) => match topo.sound_processor(spid) {
                        Some(sp) => {
                            if !sp.number_sources().contains(&nsid) {
                                return Some(NumberError::BadSourceCleanup(*nsid).into());
                            }
                        }
                        None => return Some(NumberError::BadSourceCleanup(*nsid).into()),
                    },
                    NumberSourceOwner::SoundInput(siid) => match topo.sound_input(siid) {
                        Some(si) => {
                            if !si.number_sources().contains(&nsid) {
                                return Some(NumberError::BadSourceCleanup(*nsid).into());
                            }
                        }
                        None => return Some(NumberError::BadSourceCleanup(*nsid).into()),
                    },
                }

                // the source must not be connected to any inputs
                for ni in topo.number_inputs().values() {
                    if ni.target() == Some(*nsid) {
                        return Some(NumberError::BadSourceCleanup(*nsid).into());
                    }
                }

                // the source must have no inputs
                if !data.inputs().is_empty() {
                    return Some(NumberError::BadSourceCleanup(*nsid).into());
                }
            }
            SoundGraphEdit::AddNumberInput(data) => {
                // the number input's id must not be taken
                if topo.number_input(data.id()).is_some() {
                    return Some(NumberError::InputIdTaken(data.id()).into());
                }

                // the input's owner must exist
                match data.owner() {
                    NumberInputOwner::SoundProcessor(spid) => {
                        if topo.sound_processor(spid).is_none() {
                            return Some(NumberError::BadInputInit(data.id()).into());
                        }
                    }
                    NumberInputOwner::NumberSource(nsid) => {
                        if topo.number_source(nsid).is_none() {
                            return Some(NumberError::BadInputInit(data.id()).into());
                        }
                    }
                }

                // the input must not be connected
                if data.target().is_some() {
                    return Some(NumberError::BadInputInit(data.id()).into());
                }
            }
            SoundGraphEdit::RemoveNumberInput(niid, owner_id) => {
                // the number input must exist
                let data = match topo.number_input(*niid) {
                    Some(data) => data,
                    None => return Some(NumberError::InputNotFound(*niid).into()),
                };

                // the owner must match and exist
                match owner_id {
                    NumberInputOwner::SoundProcessor(spid) => match topo.sound_processor(*spid) {
                        Some(sp) => {
                            if !sp.number_inputs().contains(niid) {
                                return Some(NumberError::BadInputCleanup(*niid).into());
                            }
                        }
                        None => return Some(NumberError::BadInputCleanup(*niid).into()),
                    },
                    NumberInputOwner::NumberSource(nsid) => match topo.number_source(*nsid) {
                        Some(ns) => {
                            if !ns.inputs().contains(niid) {
                                return Some(NumberError::BadInputCleanup(*niid).into());
                            }
                        }
                        None => return Some(NumberError::BadInputCleanup(*niid).into()),
                    },
                }

                // the number input must not be connected
                if data.target().is_some() {
                    return Some(NumberError::BadInputCleanup(*niid).into());
                }
            }
            SoundGraphEdit::ConnectNumberInput(niid, nsid) => {
                // the number input must exist
                if topo.number_input(*niid).is_none() {
                    return Some(NumberError::InputNotFound(*niid).into());
                }

                // the number source must exist
                if topo.number_source(*nsid).is_none() {
                    return Some(NumberError::SourceNotFound(*nsid).into());
                }

                // the number input must be vacant
                if let Some(target) = topo.number_input(*niid).unwrap().target() {
                    return Some(
                        NumberError::InputOccupied {
                            input_id: *niid,
                            current_target: target,
                        }
                        .into(),
                    );
                }

                // the connection must be legal
                if let Err(e) = validate_number_connection(topo, *niid, *nsid) {
                    return Some(e);
                }
            }
            SoundGraphEdit::DisconnectNumberInput(niid) => {
                // the number input must exist
                let data = match topo.number_input(*niid) {
                    Some(data) => data,
                    None => return Some(NumberError::InputNotFound(*niid).into()),
                };

                // the number input must be occupied
                if data.target().is_none() {
                    return Some(NumberError::InputUnoccupied(*niid).into());
                }
            }
        }
        None
    }
}
