use std::time::{Duration, Instant};

use crate::core::graphobject::GraphId;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticRelevance {
    Primary,
    Secondary,
}

pub enum DiagnosticMessage {
    GenericWarning((GraphId, DiagnosticRelevance)),
}

impl DiagnosticMessage {
    pub fn involves(&self, graph_id: GraphId) -> Option<DiagnosticRelevance> {
        match self {
            DiagnosticMessage::GenericWarning((id, r)) => {
                if *id == graph_id {
                    Some(*r)
                } else {
                    None
                }
            }
        }
    }
}

pub struct Diagnostic {
    message: DiagnosticMessage,
    time_issued: Instant,
}

impl Diagnostic {
    pub fn new(message: DiagnosticMessage) -> Diagnostic {
        Diagnostic {
            message,
            time_issued: Instant::now(),
        }
    }

    pub fn message(&self) -> &DiagnosticMessage {
        &self.message
    }
}

pub struct AllDiagnostics {
    diagnostics: Vec<Diagnostic>,
}

impl AllDiagnostics {
    pub(super) fn new() -> AllDiagnostics {
        AllDiagnostics {
            diagnostics: Vec::new(),
        }
    }

    pub fn push_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn get_diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn age_out(&mut self, max_age: Duration) {
        let now = Instant::now();
        self.diagnostics.retain(|d| {
            let age = now.duration_since(d.time_issued);
            age <= max_age
        });
    }

    pub fn graph_item_has_warning(&self, graph_id: GraphId) -> Option<DiagnosticRelevance> {
        let mut r = None;
        for d in &self.diagnostics {
            if let Some(rr) = d.message().involves(graph_id) {
                if rr == DiagnosticRelevance::Primary || r.is_none() {
                    r = Some(rr);
                }
            }
        }
        r
    }
}
