use super::{
    numberinput::{NumberInputId, NumberInputOwner},
    numbersource::{NumberSourceId, NumberSourceOwner},
    soundgraphdata::{NumberInputData, NumberSourceData, SoundInputData, SoundProcessorData},
    soundgraphtopology::SoundGraphTopology,
    soundgraphvalidation::{
        validate_number_connection, validate_sound_connection, validate_sound_disconnection,
    },
    soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
};

#[derive(Clone)]
pub(super) enum SoundGraphEdit {
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
    pub(super) fn check_preconditions(&self, topo: &SoundGraphTopology) -> bool {
        match self {
            SoundGraphEdit::AddSoundProcessor(data) => {
                // The processor id must not be taken
                if topo.sound_processor(data.id()).is_some() {
                    println!("AddSoundProcessor: The processor id already exists");
                    return false;
                }

                // the processor must have no sound inputs
                if data.sound_inputs().len() > 0 {
                    println!("AddSoundProcessor: The processor must have no sound inputs");
                    return false;
                }

                // the processor must have no number sources
                if data.number_sources().len() > 0 {
                    println!("AddSoundProcessor: The processor must have no number sources");
                    return false;
                }

                // the processor must have no number inputs
                if data.number_inputs().len() > 0 {
                    println!("AddSoundProcessor: The processor must have no number inputs");
                    return false;
                }
            }
            SoundGraphEdit::RemoveSoundProcessor(spid) => {
                // the processor must exist
                let data = match topo.sound_processor(*spid) {
                    Some(data) => data,
                    None => {
                        println!("RemoveSoundProcessor: The processor must exist");
                        return false;
                    }
                };

                // it may not be connected to any sound inputs
                for si in topo.sound_inputs().values() {
                    if si.target() == Some(*spid) {
                        println!("RemoveSoundProcessor: The processor must not be connected to any sound inputs");
                        return false;
                    }
                }

                // all its sound inputs must be removed
                if !data.sound_inputs().is_empty() {
                    println!("RemoveSoundProcessor: The processor must have no sound inputs");
                    return false;
                }

                // all its number sources must be removed
                if !data.number_sources().is_empty() {
                    println!("RemoveSoundProcessor: The processor must have no number sources");
                    return false;
                }

                // all number inputs must be disconnected
                if !data.number_inputs().is_empty() {
                    println!("RemoveSoundProcessor: The processor must have no number inputs");
                    return false;
                }
            }
            SoundGraphEdit::AddSoundInput(data) => {
                // the input id must not be taken
                if topo.sound_input(data.id()).is_some() {
                    println!("AddSoundInput: The input id already exists");
                    return false;
                }

                // the owner processor must exist
                if topo.sound_processor(data.owner()).is_none() {
                    println!("AddSoundInput: The sound input owner must exist");
                    return false;
                }

                // the input must be vacant
                if data.target().is_some() {
                    println!("AddSoundInput: The input must be vacant");
                    return false;
                }

                // the input must have no number sources
                if data.number_sources().len() > 0 {
                    println!("AddSoundInput: The sound input must have no number sources");
                    return false;
                }
            }
            SoundGraphEdit::RemoveSoundInput(siid, owner_spid) => {
                // the sound input must exist
                let data = match topo.sound_input(*siid) {
                    Some(data) => data,
                    None => {
                        println!("RemoveSoundInput: The sound input must exist");
                        return false;
                    }
                };

                // the sound input's owner must match and exist
                if *owner_spid != data.owner() || topo.sound_processor(*owner_spid).is_none() {
                    println!("RemoveSoundInput: The sound input owner must match and exist");
                    return false;
                }

                // the sound input must not be connected
                if data.target().is_some() {
                    println!("RemoveSoundInput: The sound input owner must match and exist");
                    return false;
                }

                // the sound input must have no number sources
                if data.number_sources().len() > 0 {
                    println!("RemoveSoundInput: The sound input must have no number sources");
                    return false;
                }
            }
            SoundGraphEdit::AddSoundInputKey(siid, index) => {
                // the sound input must exist
                let data = match topo.sound_input(*siid) {
                    Some(data) => data,
                    None => {
                        println!("AddSoundInputKey: the sound input must exist");
                        return false;
                    }
                };

                // the index must be at most num_keys
                if *index > data.num_keys() {
                    println!("AddSoundInputKey: the key index must be within range");
                    return false;
                }
            }
            SoundGraphEdit::RemoveSoundInputKey(siid, index) => {
                // the sound input must exist
                let data = match topo.sound_input(*siid) {
                    Some(data) => data,
                    None => {
                        println!("RemoveSoundInputKey: the sound input must exist");
                        return false;
                    }
                };

                // the index must be at most num_keys-1
                if *index >= data.num_keys() {
                    println!("RemoveSoundInputKey: the key index must be within range");
                    return false;
                }
            }
            SoundGraphEdit::ConnectSoundInput(siid, spid) => {
                // the input must exist
                let data = match topo.sound_input(*siid) {
                    Some(data) => data,
                    None => {
                        println!("ConnectSoundInput: the sound input must exist");
                        return false;
                    }
                };

                // the processor must exist
                if topo.sound_processor(*spid).is_none() {
                    println!("ConnectSoundInput: the target sound processor must exist");
                    return false;
                }

                // the input must be vacant
                if data.target().is_some() {
                    println!("ConnectSoundInput: the sound input must be vacant");
                    return false;
                }

                // the connection must be legal
                if validate_sound_connection(topo, *siid, *spid).is_err() {
                    println!("ConnectSoundInput: the connection must be legal");
                    return false;
                }
            }
            SoundGraphEdit::DisconnectSoundInput(siid) => {
                // the sound input must exist
                let data = match topo.sound_input(*siid) {
                    Some(data) => data,
                    None => {
                        println!("DisconnectSoundInput: the sound input must exist");
                        return false;
                    }
                };

                // the sound input must be occupied
                if data.target().is_none() {
                    println!("DisconnectSoundInput: the sound input must be occupied");
                    return false;
                }

                // the sound input must be safe to disconnect
                if validate_sound_disconnection(topo, *siid).is_err() {
                    println!("DisconnectSoundInput: the sound input must be safe to disconnect");
                    return false;
                }
            }
            SoundGraphEdit::AddNumberSource(data) => {
                // the source's id must not be taken
                if topo.number_source(data.id()).is_some() {
                    return false;
                }

                // the source's owner must exist
                match data.owner() {
                    NumberSourceOwner::Nothing => (),
                    NumberSourceOwner::SoundProcessor(spid) => {
                        if topo.sound_processor(spid).is_none() {
                            return false;
                        }
                    }
                    NumberSourceOwner::SoundInput(siid) => {
                        if topo.sound_input(siid).is_none() {
                            return false;
                        }
                    }
                }

                // the source must have no inputs
                if data.inputs().len() > 0 {
                    return false;
                }
            }
            SoundGraphEdit::RemoveNumberSource(nsid, owner_id) => {
                // the source must exist
                let data = match topo.number_source(*nsid) {
                    Some(data) => data,
                    None => return false,
                };

                // the owner must match and exist
                if *owner_id != data.owner() {
                    return false;
                }
                match *owner_id {
                    NumberSourceOwner::Nothing => (),
                    NumberSourceOwner::SoundProcessor(spid) => match topo.sound_processor(spid) {
                        Some(sp) => {
                            if !sp.number_sources().contains(&nsid) {
                                return false;
                            }
                        }
                        None => return false,
                    },
                    NumberSourceOwner::SoundInput(siid) => match topo.sound_input(siid) {
                        Some(si) => {
                            if !si.number_sources().contains(&nsid) {
                                return false;
                            }
                        }
                        None => return false,
                    },
                }

                // the source must not be connected to any inputs
                for ni in topo.number_inputs().values() {
                    if ni.target() == Some(*nsid) {
                        return false;
                    }
                }

                // the source must have no inputs
                if data.inputs().len() > 0 {
                    return false;
                }
            }
            SoundGraphEdit::AddNumberInput(data) => {
                // the number input's id must not be taken
                if topo.number_input(data.id()).is_some() {
                    return false;
                }

                // the input's owner must exist
                match data.owner() {
                    NumberInputOwner::SoundProcessor(spid) => {
                        if topo.sound_processor(spid).is_none() {
                            return false;
                        }
                    }
                    NumberInputOwner::NumberSource(nsid) => {
                        if topo.number_source(nsid).is_none() {
                            return false;
                        }
                    }
                }

                // the input must not be connected
                if data.target().is_some() {
                    return false;
                }
            }
            SoundGraphEdit::RemoveNumberInput(niid, owner_id) => {
                // the number input must exist
                let data = match topo.number_input(*niid) {
                    Some(data) => data,
                    None => return false,
                };

                // the owner must match and exist
                match owner_id {
                    NumberInputOwner::SoundProcessor(spid) => match topo.sound_processor(*spid) {
                        Some(sp) => {
                            if !sp.number_inputs().contains(niid) {
                                return false;
                            }
                        }
                        None => return false,
                    },
                    NumberInputOwner::NumberSource(nsid) => match topo.number_source(*nsid) {
                        Some(ns) => {
                            if !ns.inputs().contains(niid) {
                                return false;
                            }
                        }
                        None => return false,
                    },
                }

                // the number input must not be connected
                if data.target().is_some() {
                    return false;
                }
            }
            SoundGraphEdit::ConnectNumberInput(niid, nsid) => {
                // the number input must exist
                if topo.number_input(*niid).is_none() {
                    return false;
                }

                // the number source must exist
                if topo.number_source(*nsid).is_none() {
                    return false;
                }

                // the number input must be vacant
                if topo.number_input(*niid).unwrap().target().is_some() {
                    return false;
                }

                // the connection must be legal
                if validate_number_connection(topo, *niid, *nsid).is_err() {
                    return false;
                }
            }
            SoundGraphEdit::DisconnectNumberInput(niid) => {
                // the number input must exist
                let data = match topo.number_input(*niid) {
                    Some(data) => data,
                    None => return false,
                };

                // the number input must be occupied
                if data.target().is_none() {
                    return false;
                }
            }
        }
        true
    }
}
