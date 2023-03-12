use std::sync::Arc;

use parking_lot::RwLock;

use crate::core::{
    context::Context,
    graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    numbersource::{NumberSourceHandle, NumberVisibility},
    samplefrequency::SAMPLE_FREQUENCY,
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundinputtypes::{KeyReuse, KeyedInputQueue, KeyedInputQueueNode},
    soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
    soundprocessortools::SoundProcessorTools,
    state::State,
    uniqueid::{IdGenerator, UniqueId},
};

use super::functions::Divide;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoteId(usize);

impl UniqueId for NoteId {
    fn value(&self) -> usize {
        self.0
    }

    fn next(&self) -> Self {
        NoteId(self.0 + 1)
    }
}

impl Default for NoteId {
    fn default() -> Self {
        NoteId(1)
    }
}

pub struct NoteState {
    frequency: f32,
    length_seconds: f32,
}

impl State for NoteState {
    fn reset(&mut self) {
        self.frequency = 0.0;
        self.length_seconds = 0.0;
    }
}

#[derive(Clone, Copy)]
pub struct Note {
    pub start_time_samples: usize,
    pub duration_samples: usize,
    pub frequency: f32,
}

// Stuff that is needed during audio processing but can be changed live
struct MelodyData {
    length_samples: usize,
    notes: Vec<(NoteId, Note)>,
    note_idgen: IdGenerator<NoteId>,
}

pub struct Melody {
    shared_data: Arc<RwLock<MelodyData>>,
    pub input: KeyedInputQueue<NoteId, NoteState>,
    pub melody_time: NumberSourceHandle,
    pub note_frequency: NumberSourceHandle,
    pub note_time: NumberSourceHandle,
    _note_length: NumberSourceHandle,
    pub note_progress: NumberSourceHandle,
}

pub struct MelodyState {
    current_position: usize,
    shared_data: Arc<RwLock<MelodyData>>,
}

impl State for MelodyState {
    fn reset(&mut self) {
        self.current_position = 0;
    }
}

impl Melody {
    pub fn length_samples(&self) -> usize {
        self.shared_data.read().length_samples
    }

    pub fn add_note(&self, note: Note) -> NoteId {
        let id;
        {
            let mut data = self.shared_data.write();
            id = data.note_idgen.next_id();
            data.notes.push((id, note));
        };
        id
    }

    pub fn remove_note(&self, note_id: NoteId) {
        let mut data = self.shared_data.write();
        data.notes.retain(|(id, _n)| *id != note_id);
    }

    pub fn edit_note(&self, id: NoteId, updated_note: Note) {
        let mut data = self.shared_data.write();
        for (note_id, note) in &mut data.notes {
            if *note_id == id {
                *note = updated_note;
                break;
            }
        }
    }

    pub fn notes(&self) -> Vec<(NoteId, Note)> {
        self.shared_data.read().notes.clone()
    }

    pub fn set_notes(&self, notes: Vec<(NoteId, Note)>) {
        self.shared_data.write().notes = notes;
    }

    pub fn clear(&self) {
        self.shared_data.write().notes.clear();
    }
}

impl DynamicSoundProcessor for Melody {
    type StateType = MelodyState;

    type SoundInputType = KeyedInputQueue<NoteId, NoteState>;

    type NumberInputType<'ctx> = ();

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        let queue_size = 8; // idk
        let input = KeyedInputQueue::new(queue_size, &mut tools);
        let note_frequency = tools.add_input_scalar_number_source(
            input.id(),
            |state| state.downcast_if::<NoteState>().unwrap().frequency,
            NumberVisibility::Public,
        );
        let note_time = tools.add_input_time(input.id(), NumberVisibility::Public);
        let note_length = tools.add_input_scalar_number_source(
            input.id(),
            |state| state.downcast_if::<NoteState>().unwrap().length_seconds,
            NumberVisibility::Private,
        );
        let note_progress = tools
            .add_derived_input_number_source::<Divide>(input.id(), NumberVisibility::Public)
            .unwrap();

        tools.connect_number_input(note_progress.input_1.id(), note_time.id());
        tools.connect_number_input(note_progress.input_2.id(), note_length.id());

        Ok(Melody {
            shared_data: Arc::new(RwLock::new(MelodyData {
                length_samples: SAMPLE_FREQUENCY * 4,
                notes: Vec::new(),
                note_idgen: IdGenerator::new(),
            })),
            input,
            melody_time: tools.add_processor_time(NumberVisibility::Public),
            note_frequency,
            note_time,
            _note_length: note_length,
            note_progress: note_progress.into(),
        })
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &self.input
    }

    fn make_state(&self) -> Self::StateType {
        MelodyState {
            current_position: 0,
            shared_data: Arc::clone(&self.shared_data),
        }
    }

    fn make_number_inputs<'ctx>(
        &self,
        _context: &'ctx inkwell::context::Context,
    ) -> Self::NumberInputType<'ctx> {
        ()
    }

    fn process_audio<'ctx>(
        state: &mut StateAndTiming<Self::StateType>,
        sound_input: &mut KeyedInputQueueNode<NoteId, NoteState>,
        _number_inputs: &(),
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus {
        let length_samples;
        {
            let data = state.shared_data.read();

            length_samples = data.length_samples;

            for (note_id, note) in &data.notes {
                let start_of_chunk = state.current_position;
                let end_of_chunk = state.current_position + CHUNK_SIZE;

                let mut start_offset: Option<usize> = None;

                if note.start_time_samples >= start_of_chunk
                    && note.start_time_samples < end_of_chunk
                {
                    start_offset = Some(note.start_time_samples - start_of_chunk);
                } else if end_of_chunk > data.length_samples {
                    let wraparound = end_of_chunk - data.length_samples;
                    if note.start_time_samples < wraparound {
                        start_offset = Some(CHUNK_SIZE - wraparound + note.start_time_samples);
                    }
                }

                if let Some(offset) = start_offset {
                    // TODO: use offset when queueing note
                    sound_input.start_key(
                        Some(note.duration_samples),
                        *note_id,
                        NoteState {
                            frequency: note.frequency,
                            length_seconds: note.duration_samples as f32 / SAMPLE_FREQUENCY as f32,
                        },
                        KeyReuse::StopOldStartNew,
                    );
                }
            }
        }

        debug_assert!(length_samples >= CHUNK_SIZE);
        state.current_position += CHUNK_SIZE;
        if state.current_position >= length_samples {
            state.current_position -= length_samples;
        }

        sound_input.step(state, dst, &context)
    }
}

impl WithObjectType for Melody {
    const TYPE: ObjectType = ObjectType::new("melody");
}
