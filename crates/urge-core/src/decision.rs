//! Verdict and audit-trail types.
//!
//! Every evaluation produces a [`Verdict`] that is fully self-describing:
//! you can reconstruct *exactly* why the governance engine reached its conclusion
//! by walking the [`LogicTrace`].

use crate::engine::Paradigm;
use crate::symbol::ParadigmSet;

/// The outcome of a complete governance evaluation.
///
/// This is the output of the Figure 26 pipeline.
#[derive(Debug, Clone)]
// Serialize-only: holds `&'static str` fields for zero-alloc use, which cannot Deserialize.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Verdict {
    /// Whether the evaluated expression is valid / permitted / satisfied.
    pub valid: bool,

    /// Overall confidence score [0.0, 1.0].
    ///
    /// Derived from the agreement ratio across paradigm engines:
    ///   1.0 = all active engines agree
    ///   0.5 = split across paradigms
    ///   0.0 = complete disagreement (paraconsistent scenario)
    pub confidence: Confidence,

    /// Which paradigms were active and evaluated.
    pub paradigms_evaluated: ParadigmSet,

    /// The full audit trail — every routing and evaluation decision.
    pub trace: LogicTrace,

    /// Cross-validation result from the meta-engine.
    pub cross_validation: CrossValidation,

    /// Formal notation of the evaluated expression (Unicode logic symbols).
    #[cfg(feature = "alloc")]
    pub formal_notation: alloc::string::String,

    /// Policy citations — the authoritative sources that justify the verdict.
    #[cfg(feature = "alloc")]
    pub citations: alloc::vec::Vec<Citation>,
}

impl Verdict {
    /// Construct a fast-path denial with minimal tracing.
    /// Used for hard-coded prohibitions (e.g., in the BIOS access-control path).
    pub fn deny_immediate(reason: &'static str) -> Self {
        let mut trace = LogicTrace::new();
        trace.push(TraceEntry {
            stage: Stage::CrossValidation,
            paradigm: Some(Paradigm::Deontic),
            description: reason,
            outcome: EntryOutcome::Denied,
        });
        Verdict {
            valid: false,
            confidence: Confidence::CERTAIN,
            paradigms_evaluated: {
                let mut s = ParadigmSet::empty();
                s.insert(Paradigm::Deontic);
                s
            },
            trace,
            cross_validation: CrossValidation {
                consistent: false,
                conflicts_detected: 1,
                conflict_detail: Some(reason),
            },
            #[cfg(feature = "alloc")]
            formal_notation: alloc::format!("¬permitted({reason})"),
            #[cfg(feature = "alloc")]
            citations: alloc::vec![],
        }
    }

    /// Construct a fast-path permit.
    pub fn permit_immediate(reason: &'static str) -> Self {
        let mut trace = LogicTrace::new();
        trace.push(TraceEntry {
            stage: Stage::CrossValidation,
            paradigm: Some(Paradigm::Deontic),
            description: reason,
            outcome: EntryOutcome::Permitted,
        });
        Verdict {
            valid: true,
            confidence: Confidence::CERTAIN,
            paradigms_evaluated: {
                let mut s = ParadigmSet::empty();
                s.insert(Paradigm::Deontic);
                s
            },
            trace,
            cross_validation: CrossValidation {
                consistent: true,
                conflicts_detected: 0,
                conflict_detail: None,
            },
            #[cfg(feature = "alloc")]
            formal_notation: alloc::format!("permitted({reason})"),
            #[cfg(feature = "alloc")]
            citations: alloc::vec![],
        }
    }
}

// ── Confidence ─────────────────────────────────────────────────────────────────

/// A confidence score in the range [0, 255] mapped to [0.0, 1.0].
/// Stored as u8 to avoid float in no_std environments where soft-float is slow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Confidence(pub u8);

impl Confidence {
    pub const CERTAIN: Confidence = Confidence(255);
    pub const HIGH: Confidence = Confidence(204); // ~0.80
    pub const MEDIUM: Confidence = Confidence(153); // ~0.60
    pub const LOW: Confidence = Confidence(102); // ~0.40
    pub const UNCERTAIN: Confidence = Confidence(51); // ~0.20
    pub const NONE: Confidence = Confidence(0);

    /// Compute from agreement ratio: `agreed` engines out of `total`.
    pub fn from_agreement(agreed: u8, total: u8) -> Self {
        if total == 0 {
            return Confidence::NONE;
        }
        Confidence(((agreed as u16 * 255) / total as u16) as u8)
    }

    pub fn as_f32(self) -> f32 {
        self.0 as f32 / 255.0
    }
}

// ── CrossValidation ─────────────────────────────────────────────────────────────

/// Result of the cross-system validation stage (Step 3 of Figure 26).
///
/// This is the key differentiation: the meta-engine checks that results from
/// multiple paradigm engines are logically consistent with each other before
/// producing a final verdict.
#[derive(Debug, Clone)]
// Serialize-only: holds `&'static str` fields for zero-alloc use, which cannot Deserialize.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CrossValidation {
    /// True if all paradigm engines reached compatible conclusions.
    pub consistent: bool,
    /// Number of inter-paradigm conflicts detected.
    pub conflicts_detected: u8,
    /// Description of the first conflict found, if any.
    pub conflict_detail: Option<&'static str>,
}

impl CrossValidation {
    pub fn ok() -> Self {
        CrossValidation {
            consistent: true,
            conflicts_detected: 0,
            conflict_detail: None,
        }
    }
}

// ── LogicTrace ─────────────────────────────────────────────────────────────────

/// The complete audit trail for one governance evaluation.
///
/// Bounded to 64 entries on embedded targets. On `std` targets this can grow.
#[derive(Debug, Clone)]
// Serialize-only: holds `&'static str` fields (via TraceEntry) for zero-alloc use, which cannot Deserialize.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct LogicTrace {
    #[cfg(feature = "alloc")]
    pub entries: alloc::vec::Vec<TraceEntry>,
    #[cfg(not(feature = "alloc"))]
    pub entries: heapless::Vec<TraceEntry, 64>,
}

impl LogicTrace {
    pub fn new() -> Self {
        LogicTrace {
            #[cfg(feature = "alloc")]
            entries: alloc::vec::Vec::new(),
            #[cfg(not(feature = "alloc"))]
            entries: heapless::Vec::new(),
        }
    }

    pub fn push(&mut self, entry: TraceEntry) {
        #[cfg(feature = "alloc")]
        self.entries.push(entry);
        #[cfg(not(feature = "alloc"))]
        let _ = self.entries.push(entry);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for LogicTrace {
    fn default() -> Self {
        Self::new()
    }
}

/// A single step in the logic trace.
#[derive(Debug, Clone)]
// Serialize-only: `description` is `&'static str` for zero-alloc use, which cannot Deserialize.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct TraceEntry {
    /// Which pipeline stage produced this entry.
    pub stage: Stage,
    /// The paradigm involved, if applicable.
    pub paradigm: Option<Paradigm>,
    /// Human-readable description (static string for zero-allocation on embedded).
    pub description: &'static str,
    /// The outcome of this step.
    pub outcome: EntryOutcome,
}

/// Pipeline stages (maps to the Figure 26 workflow steps).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Stage {
    Tokenization,
    ParadigmDetection,
    AstConstruction,
    EngineRouting,
    EngineEvaluation,
    CrossValidation,
    VerdictSynthesis,
}

/// Outcome of a trace step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EntryOutcome {
    Permitted,
    Denied,
    Evaluated,
    Routed,
    Conflict,
    Skipped,
}

/// A regulatory or policy citation anchoring a governance decision.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Citation {
    /// Short identifier, e.g. "HIPAA-§164.312(a)"
    pub id: alloc::string::String,
    /// Human-readable description of the cited requirement.
    pub description: alloc::string::String,
}
