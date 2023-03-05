use std::time::{Duration, Instant};

use crate::core::{
    graphobject::GraphId,
    soundgrapherror::{NumberError, SoundError, SoundGraphError},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticRelevance {
    Primary,
    Secondary,
}

pub enum DiagnosticMessage {
    GraphItemWarning((GraphId, DiagnosticRelevance)),
}

impl DiagnosticMessage {
    pub fn involves(&self, graph_id: GraphId) -> Option<DiagnosticRelevance> {
        match self {
            DiagnosticMessage::GraphItemWarning((id, r)) => {
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

    pub fn push_interpreted_error(&mut self, error: SoundGraphError) {
        match error {
            SoundGraphError::Number(e) => self.push_interpreted_number_error(e),
            SoundGraphError::Sound(e) => self.push_interpreted_sound_error(e),
        }
    }

    fn push_interpreted_number_error(&mut self, error: NumberError) {
        match error {
            NumberError::CircularDependency { cycle } => {
                for (nsid, niid) in cycle.connections {
                    self.push_diagnostic(Diagnostic::new(DiagnosticMessage::GraphItemWarning((
                        nsid.into(),
                        DiagnosticRelevance::Secondary,
                    ))));
                    self.push_diagnostic(Diagnostic::new(DiagnosticMessage::GraphItemWarning((
                        niid.into(),
                        DiagnosticRelevance::Secondary,
                    ))));
                }
            }
            NumberError::StateNotInScope { bad_dependencies } => {
                for (nsid, niid) in bad_dependencies {
                    self.push_diagnostic(Diagnostic::new(DiagnosticMessage::GraphItemWarning((
                        nsid.into(),
                        DiagnosticRelevance::Secondary,
                    ))));
                    self.push_diagnostic(Diagnostic::new(DiagnosticMessage::GraphItemWarning((
                        niid.into(),
                        DiagnosticRelevance::Secondary,
                    ))));
                }
            }
            // Other errors are assumed to be internal and never caused by the user
            _ => (),
        }
    }

    fn push_interpreted_sound_error(&mut self, error: SoundError) {
        match error {
            SoundError::CircularDependency { cycle } => {
                for (spid, siid) in cycle.connections {
                    self.push_diagnostic(Diagnostic::new(DiagnosticMessage::GraphItemWarning((
                        spid.into(),
                        DiagnosticRelevance::Secondary,
                    ))));
                    self.push_diagnostic(Diagnostic::new(DiagnosticMessage::GraphItemWarning((
                        siid.into(),
                        DiagnosticRelevance::Secondary,
                    ))));
                }
            }
            SoundError::StaticTooManyStates(spid) => {
                self.push_diagnostic(Diagnostic::new(DiagnosticMessage::GraphItemWarning((
                    spid.into(),
                    DiagnosticRelevance::Primary,
                ))));
            }
            SoundError::StaticNotSynchronous(spid) => {
                self.push_diagnostic(Diagnostic::new(DiagnosticMessage::GraphItemWarning((
                    spid.into(),
                    DiagnosticRelevance::Primary,
                ))));
            }
            // Other errors are assumed to be internal and never caused by the user
            _ => (),
        }
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
