use std::sync::Arc;

use parking_lot::{Mutex, RwLock};

use crate::core::{
    engine::nodegen::NodeGen,
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    samplefrequency::SAMPLE_FREQUENCY,
    sound::{
        context::{Context, LocalArrayList},
        expressionargument::SoundExpressionArgumentHandle,
        soundinputtypes::{KeyReuse, KeyedInputQueue, KeyedInputQueueNode},
        soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
        soundprocessortools::SoundProcessorTools,
        state::State,
    },
    soundchunk::{SoundChunk, CHUNK_SIZE},
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

// TODO: custom per-note variables

// Stuff that is needed during audio processing but can be changed live
struct MelodyData {
    length_samples: usize,
    notes: Vec<(NoteId, Note)>,
}

pub struct Melody {
    shared_data: Arc<RwLock<MelodyData>>,
    note_idgen: Mutex<IdGenerator<NoteId>>,
    pub input: KeyedInputQueue<NoteState>,
    pub note_frequency: SoundExpressionArgumentHandle,
    _note_length: SoundExpressionArgumentHandle,
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
        let id = self.note_idgen.lock().next_id();
        {
            let mut data = self.shared_data.write();
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

    type SoundInputType = KeyedInputQueue<NoteState>;

    type Expressions<'ctx> = ();

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        let queue_size = 8; // idk
        let input = KeyedInputQueue::new(queue_size, &mut tools);
        let note_frequency = tools.add_input_scalar_argument(input.id(), |state| {
            state.downcast_if::<NoteState>().unwrap().frequency
        });
        let note_length = tools.add_input_scalar_argument(input.id(), |state| {
            state.downcast_if::<NoteState>().unwrap().length_seconds
        });
        // TODO: add note progress (time / length) as a derived number source

        Ok(Melody {
            shared_data: Arc::new(RwLock::new(MelodyData {
                length_samples: SAMPLE_FREQUENCY * 4,
                notes: Vec::new(),
            })),
            note_idgen: Mutex::new(IdGenerator::new()),
            input,
            note_frequency,
            _note_length: note_length,
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

    fn compile_expressions<'a, 'ctx>(
        &self,
        _nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        ()
    }

    fn process_audio<'ctx>(
        state: &mut StateAndTiming<Self::StateType>,
        sound_input: &mut KeyedInputQueueNode<NoteState>,
        _number_inputs: &mut (),
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus {
        // TODO: stop looping if released

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
                        note_id.0,
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

        sound_input.step(state, dst, &context, LocalArrayList::new());
        StreamStatus::Playing
    }
}

impl WithObjectType for Melody {
    const TYPE: ObjectType = ObjectType::new("melody");
}
