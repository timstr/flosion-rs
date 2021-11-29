use crate::sound::soundstate::{EmptyState, SoundState};
use crate::sound::statetable::{KeyedStateTable, StateTable};
use crate::sound::uniqueid::UniqueId;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SoundInputId(usize);

impl Default for SoundInputId {
    fn default() -> SoundInputId {
        SoundInputId(1)
    }
}

impl UniqueId for SoundInputId {
    fn value(&self) -> usize {
        self.0
    }
    fn next(&self) -> SoundInputId {
        SoundInputId(self.0 + 1)
    }
}

#[derive(Copy, Clone)]
pub struct InputOptions {
    // Will the input ever be paused or reset by the sound processor?
    pub interruptible: bool,

    // Will the input's speed of time always be the same as the sound processor's?
    pub realtime: bool,
}

// TODO: data needed for sound inputs:
// single inputs
//     - time offset (at start of chunk?) relative to sound processor
//     - time speed (at start of chunk?) relative to sound processor
//     - no additional per-state data (the sound processor's state will always suffice here)
// keyed inputs:
//     - list of keys (just use usize in the basic interface, map these to custom types somehow when needed)
//     - for each key, the time offset and time speed as above
//     - arbitrary per-key data (e.g. note envelope spline)
//     - arbitrary per-state data (e.g. ensemble note frequency offset)
// for convenience, a single input may be considered to be a keyed input that always has exactly one key

// TODO:
// - Sound processors own their inputs (SingleSoundInput and KeyedSoundInput<K, T>, see below) but they
//   can't do anything with them directly
// - Creating and modifying inputs requires tools from the soundgraph
// - Accessing the (writable) state and (readonly) key data of an input is achieved by
//   passing the strongly-typed (in the case of keyed inputs) input wrapper to the
//   audio processing context
// - the same soundgraph tools needed for modifying the inputs registers the inputs with the
//   sound graph which assumes responsibility for routing and updating states, etc
// - all in all, the sound processor interface is again concerned **only** with DSP calculations,
//   and is simply handed all the pieces it needs by the context object

pub struct SingleSoundInput {
    state_table: StateTable<EmptyState>,
}

pub struct KeyedSoundInput<K: Ord, T: SoundState> {
    state_table: KeyedStateTable<K, T>,
}
