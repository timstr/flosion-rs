use std::collections::HashMap;

use crate::core::{
    engine::compiledprocessor::{AnyCompiledProcessorData, CompiledProcessorLink},
    sound::soundprocessor::{CompiledComponentVisitor, SoundProcessorId},
    soundchunk::CHUNK_SIZE,
};

use super::{compiledprocessor::CompiledSoundInputNode, compiledsoundgraph::CompiledSoundGraph};

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

        struct Visitor<'a> {
            report: &'a mut SoundEngineReport,
        }

        impl<'a> Visitor<'a> {
            fn processor(&mut self, processor: &dyn AnyCompiledProcessorData) {
                let elapsed_samples = processor.timing().elapsed_chunks() * CHUNK_SIZE;
                let proc_report =
                    self.report
                        .processors
                        .entry(processor.id())
                        .or_insert_with(|| CompiledProcessorReport {
                            times_samples: Vec::new(),
                        });
                proc_report.times_samples.push(elapsed_samples);

                processor.visit(self);
            }
        }

        impl<'a> CompiledComponentVisitor for Visitor<'a> {
            fn input_node(&mut self, input: &CompiledSoundInputNode) {
                match input.link() {
                    CompiledProcessorLink::Unique(node) => self.processor(node.processor()),
                    CompiledProcessorLink::Shared(node) => {
                        self.processor(node.borrow_cache().processor())
                    }
                    CompiledProcessorLink::Empty => (),
                }
            }
        }

        let mut visitor = Visitor { report: self };

        for node in compiled_graph.static_processors() {
            if node.is_entry_point() {
                visitor.processor(node.borrow_cache().processor());
            }
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
