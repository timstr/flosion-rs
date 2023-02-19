use std::sync::Arc;

use parking_lot::RwLock;

use crate::core::{
    context::Context,
    graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    numbersource::StateNumberSourceHandle,
    samplefrequency::SAMPLE_FREQUENCY,
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundinputtypes::{KeyReuse, KeyedInputQueue, KeyedInputQueueNode},
    soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
    soundprocessortools::SoundProcessorTools,
    state::State,
    uniqueid::{IdGenerator, UniqueId},
};

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
}

impl State for NoteState {
    fn reset(&mut self) {
        self.frequency = 0.0;
    }
}

#[derive(Clone)]
pub struct Note {
    pub id: NoteId,
    pub start_time_samples: usize,
    pub duration_samples: usize,
    pub frequency: f32,
}

// Stuff that is needed during audio processing but can be changed live
struct MelodyData {
    length_samples: usize,
    notes: Vec<Note>,
    note_idgen: IdGenerator<NoteId>,
}

pub struct Melody {
    shared_data: Arc<RwLock<MelodyData>>,
    pub input: KeyedInputQueue<NoteId, NoteState>,
    pub note_frequency: StateNumberSourceHandle,
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

    pub fn add_note(
        &self,
        start_time_samples: usize,
        duration_samples: usize,
        frequency: f32,
    ) -> NoteId {
        let id;
        {
            let mut data = self.shared_data.write();
            id = data.note_idgen.next_id();
            data.notes.push(Note {
                id,
                start_time_samples,
                duration_samples,
                frequency,
            })
        };
        id
    }

    pub fn notes(&self) -> Vec<Note> {
        self.shared_data.read().notes.clone()
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
        let note_frequency = tools.add_input_scalar_number_source(input.id(), |state| {
            state.downcast_if::<NoteState>().unwrap().frequency
        });
        Ok(Melody {
            shared_data: Arc::new(RwLock::new(MelodyData {
                length_samples: SAMPLE_FREQUENCY * 4,
                notes: Vec::new(),
                note_idgen: IdGenerator::new(),
            })),
            input,
            note_frequency,
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

            for note in &data.notes {
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
                        note.id,
                        NoteState {
                            frequency: note.frequency,
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
