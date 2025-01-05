use std::collections::HashMap;

use crate::core::{sound::soundprocessor::SoundProcessorId, soundchunk::CHUNK_SIZE};

use super::compiledsoundgraph::CompiledSoundGraph;

pub(crate) struct CompiledProcessorReport {
    times_samples: Vec<usize>,
}

impl CompiledProcessorReport {
    /// What is the apparent current at each compiled instance of the processor, in samples?
    pub(crate) fn times_samples(&self) -> &[usize] {
        &self.times_samples
    }
}

pub(crate) struct SoundEngineReport {
    processors: HashMap<SoundProcessorId, CompiledProcessorReport>,
}

impl SoundEngineReport {
    pub(crate) fn new() -> SoundEngineReport {
        SoundEngineReport {
            processors: HashMap::new(),
        }
    }

    pub(crate) fn regenerate(&mut self, compiled_graph: &CompiledSoundGraph) {
        // Clear all samples
        for proc_report in self.processors.values_mut() {
            proc_report.times_samples.clear();
        }

        // HACK only visiting static processors because there isn't currently
        // a way to traverse the compiled graph.
        // See also todo's in verify_compiled_sound_graph
        for node in compiled_graph.static_processors() {
            let cache = node.borrow_cache();
            let elapsed_samples = cache.processor().timing().elapsed_chunks() * CHUNK_SIZE;
            let proc_report =
                self.processors
                    .entry(node.id())
                    .or_insert_with(|| CompiledProcessorReport {
                        times_samples: Vec::new(),
                    });
            proc_report.times_samples.push(elapsed_samples);
        }

        // Remove any processors without samples. These were not seen during
        // traversal and thus must have been removed.
        self.processors.retain(|_, r| !r.times_samples.is_empty());
    }

    pub(crate) fn processor_report(
        &self,
        processor_id: SoundProcessorId,
    ) -> Option<&CompiledProcessorReport> {
        self.processors.get(&processor_id)
    }
}
