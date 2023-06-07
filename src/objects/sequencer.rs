use std::sync::Arc;

use parking_lot::{Mutex, RwLock};

use crate::core::{
    samplefrequency::SAMPLE_FREQUENCY,
    sound::{
        context::Context,
        graphobject::{ObjectInitialization, ObjectType, WithObjectType},
        soundinputtypes::{KeyReuse, KeyedInputQueue, KeyedInputQueueNode},
        soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
        soundprocessortools::SoundProcessorTools,
        state::State,
    },
    soundchunk::{SoundChunk, CHUNK_SIZE},
    uniqueid::{IdGenerator, UniqueId},
};

// TODO:
// - multiple tracks (TrackId to distinguish?)
// - multiple (potentially-overlapping) start times and durations
// - custom per-track and per-snippet scalar variables

// TODO: add a sound input type with multiple KeyedInputQueues, then generalize this to multiple tracks

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SequenceItemId(usize);

impl UniqueId for SequenceItemId {
    fn value(&self) -> usize {
        self.0
    }

    fn next(&self) -> Self {
        SequenceItemId(self.0 + 1)
    }
}

impl Default for SequenceItemId {
    fn default() -> Self {
        SequenceItemId(1)
    }
}

#[derive(Copy, Clone)]
pub struct SequenceItem {
    start_time_samples: usize,
    duration_samples: usize,
}

// Stuff that is needed during audio processing but can be changed live
struct SequencerData {
    length_samples: usize,
    items: Vec<(SequenceItemId, SequenceItem)>,
}

pub struct Sequencer {
    shared_data: Arc<RwLock<SequencerData>>,
    item_idgen: Mutex<IdGenerator<SequenceItemId>>,
    input: KeyedInputQueue<SequenceItemId, ()>,
}

pub struct SequencerState {
    current_position: usize,
    shared_data: Arc<RwLock<SequencerData>>,
}

impl State for SequencerState {
    fn reset(&mut self) {
        self.current_position = 0;
    }
}

impl Sequencer {
    pub fn length_samples(&self) -> usize {
        self.shared_data.read().length_samples
    }

    pub fn add_item(&self, item: SequenceItem) -> SequenceItemId {
        let id = self.item_idgen.lock().next_id();
        {
            let mut data = self.shared_data.write();
            data.items.push((id, item));
        };
        id
    }

    pub fn remove_item(&self, item_id: SequenceItemId) {
        let mut data = self.shared_data.write();
        data.items.retain(|(id, _n)| *id != item_id);
    }

    pub fn edit_item(&self, id: SequenceItemId, updated_item: SequenceItem) {
        let mut data = self.shared_data.write();
        for (item_id, item) in &mut data.items {
            if *item_id == id {
                *item = updated_item;
                break;
            }
        }
    }

    pub fn notes(&self) -> Vec<(SequenceItemId, SequenceItem)> {
        self.shared_data.read().items.clone()
    }

    pub fn set_items(&self, items: Vec<(SequenceItemId, SequenceItem)>) {
        self.shared_data.write().items = items;
    }

    pub fn clear(&self) {
        self.shared_data.write().items.clear();
    }
}

impl DynamicSoundProcessor for Sequencer {
    type StateType = SequencerState;

    type SoundInputType = KeyedInputQueue<SequenceItemId, ()>;

    type NumberInputType<'ctx> = ();

    fn new(mut tools: SoundProcessorTools, init: ObjectInitialization) -> Result<Self, ()> {
        let queue_size = 4; // idk
        Ok(Sequencer {
            shared_data: Arc::new(RwLock::new(SequencerData {
                length_samples: SAMPLE_FREQUENCY * 4,
                items: Vec::new(),
            })),
            item_idgen: Mutex::new(IdGenerator::new()),
            input: KeyedInputQueue::new(queue_size, &mut tools),
        })
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &self.input
    }

    fn make_state(&self) -> Self::StateType {
        SequencerState {
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
        state: &mut StateAndTiming<SequencerState>,
        sound_input: &mut KeyedInputQueueNode<SequenceItemId, ()>,
        _number_inputs: &(),
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus {
        // TODO: stop looping if released

        let length_samples;
        {
            let data = state.shared_data.read();

            length_samples = data.length_samples;

            for (item_id, item) in &data.items {
                let start_of_chunk = state.current_position;
                let end_of_chunk = state.current_position + CHUNK_SIZE;

                let mut start_offset: Option<usize> = None;

                if item.start_time_samples >= start_of_chunk
                    && item.start_time_samples < end_of_chunk
                {
                    start_offset = Some(item.start_time_samples - start_of_chunk);
                } else if end_of_chunk > data.length_samples {
                    let wraparound = end_of_chunk - data.length_samples;
                    if item.start_time_samples < wraparound {
                        start_offset = Some(CHUNK_SIZE - wraparound + item.start_time_samples);
                    }
                }

                if let Some(offset) = start_offset {
                    // TODO: use offset when queueing note
                    sound_input.start_key(
                        Some(item.duration_samples),
                        *item_id,
                        (),
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

        sound_input.step(state, dst, &context);
        StreamStatus::Playing
    }
}

impl WithObjectType for Sequencer {
    const TYPE: ObjectType = ObjectType::new("sequencer");
}
