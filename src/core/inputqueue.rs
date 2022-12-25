// use std::marker::PhantomData;

// use super::{
//     soundchunk::SoundChunk,
//     statetree::{KeyedInput, State},
// };

// pub enum ReusePolicy {
//     DropNewNotes,
//     DropOldNotes,
// }

// // TODO: test this inside keyboard

// // TODO:
// // - start playing (put input onto queue)
// //     - (optional) play for fixed length?
// // - stop playing (or release, needed for keyboard and other things that can play indefinitely)
// // - what book-keeping information is needed, and conversely is any extra memory allocation needed?
// //     - ???
// // - also note that for e.g. melody, individual notes will not correspond directly to individual
// //   input keys. Instead, this queue helper will map notes to keys.
// // - Does it make sense to give the InputQueue control over the input key type? It would make
// //   book-keeping simple and direct. The melody object will want to associate information
// //   with each key (e.g. frequency, duration) that is accessible via number sources and the state
// //   context stack. Conversely, the melody object will need to be able to store information
// //   from any key with any key. An Arc to the note data might suffice for this
// // - BUT keys are (currently) already stored in an Arc
// // - AHHH I'm confusing keys with per-note-and-key input states
// // - Queue book-keeping information is obviously fit for the keyed input state
// // - What can the keys be used for, if anything? Note data will need to be stored in state
// //   (since different streams might be using the same key concurrently for different notes)
// // - Is it a bad design if the keyedinput key type is determined by the InputQueue?
// //     - No, because using a keyedinput in the first place is a decision made to enable queueing,
// //       and a 1:1 correspondence between notes and keys doesn't make sense anymore, so
// //       it seems natural to have a key type that wraps queue book-keeping and a reference to
// //       a note
// // - If Arcs to note data are used to track which input state maps to which note, then the Arc's
// //   pointer address probably suffices for tracking which notes are currently playing.
// // - removing keys (as generic types) altogether would be nice in return for all the awkward
// //   design decisions they have forced me to make here in rust world

// // historical uses for input keys:
// // - melody
// //     => keys were used to directly store each note's frequency, duration, and custom parameterized
// //        curves versus note duration. This forced the use of one key per note, which while simple
// //        to implement, prevents wrap-around, allocates more states than strictly necessary, and
// //        makes higher level queueing logic more difficult to implement. Storing pointer to note
// //        data inside state and offloading
// //     => Custom key types not needed, input states can suffice
// // - ensemble
// //     => keys were completely unused, per-voice frequency offset was stored in states.
// //     => Custom key types not needed, input states can suffice
// // - keyboard
// //     => keys were/are used to directly store whether a key is active and what its frequency is.
// //        The distinction between input key and input state is a bit meaningless here since the
// //        processor is static and so only ever has exactly one state per key. The implementation
// //        intuitively should at least roughly resemble the melody's implementation
// //     => Custom key types not needed, input states can suffice

// pub struct InputQueue<S: State + Default> {
//     _phantom_data: PhantomData<S>,
// }

// pub struct QueuedInputData<S: State + Default> {
//     state: S,
// }

// impl<S: State + Default> InputQueue<S> {
//     pub fn start_input(&self, input: &mut KeyedInput<S>, sample_offset: usize) -> usize {
//         // TODO:
//         // - use custom type implementing Eq to tell notes apart?
//         //     - what if it's desirable to have the same note (or better term for this) playing
//         //       multiple times at once, e.g. melody wraparound or pressing a keyboard key while
//         //       a note from the same key is currently being released?
//         //     - some possibilities:
//         //         - being able to release notes at any unpredictable time and being able to
//         //           play multiple notes at once are mutually exclusive by design
//         //             - this would explain the difference between keyboard and melody, but
//         //               feels like a contrived and artificial constraint
//         //         - the key type is underspecified and should distinguish this if needed
//         //             - well, if it really is the same note being played multiple times, then
//         //               no, it isn't underspecified
//         //         - the queue can track how many times a given note is already playing and
//         //           expose this through its interface
//         //         - keyboard is an exception to this queueing logic and needs to be dealt with
//         //           separately anyway
//         // - accept optional release time here?
//         todo!()
//     }

//     pub fn release_input(&self, input: &mut KeyedInput<S>, index: usize) {
//         todo!()
//     }

//     pub fn mix_active_notes(&self, input: &mut KeyedInput<S>, dst: &mut SoundChunk) {
//         todo!()
//     }
// }
