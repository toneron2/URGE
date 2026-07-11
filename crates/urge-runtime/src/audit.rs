//! Audit log — durable, append-only record of all governance decisions.
//!
//! Every `Verdict` can be logged here. The log is the external evidence that
//! the governance system operated correctly. In healthcare: the log IS the
//! compliance audit trail.

use urge_core::decision::Verdict;

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A single entry in the governance audit log.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AuditEntry {
    /// Sequence number (monotonically increasing).
    pub seq: u64,
    /// Logical timestamp (ns since epoch or monotonic counter).
    pub timestamp_ns: u64,
    /// The expression that was evaluated.
    pub expression: String,
    /// Whether the verdict permitted or denied.
    pub permitted: bool,
    /// Confidence score [0, 255].
    pub confidence: u8,
    /// Number of paradigms evaluated.
    pub paradigm_count: u8,
    /// Number of inter-paradigm conflicts detected.
    pub conflicts: u8,
    /// Formal logic notation of the evaluated expression.
    pub formal_notation: String,
    /// The full logic trace serialized to JSON (if serde feature enabled).
    #[cfg(feature = "serde")]
    pub trace_json: Option<String>,
    /// Optional correlation ID from external system (e.g., request ID, patient ID).
    pub correlation_id: Option<String>,
}

/// Append-only governance audit log.
pub struct AuditLog {
    entries: Vec<AuditEntry>,
    seq: u64,
}

impl AuditLog {
    pub fn new() -> Self {
        AuditLog {
            entries: Vec::new(),
            seq: 0,
        }
    }

    /// Record a governance decision.
    pub fn record(
        &mut self,
        expression: &str,
        verdict: &Verdict,
        timestamp_ns: u64,
        correlation_id: Option<&str>,
    ) -> u64 {
        let seq = self.seq;
        self.seq += 1;

        self.entries.push(AuditEntry {
            seq,
            timestamp_ns,
            expression: expression.into(),
            permitted: verdict.valid,
            confidence: verdict.confidence.0,
            paradigm_count: urge_core::engine::Paradigm::ALL
                .iter()
                .filter(|&&p| verdict.paradigms_evaluated.contains(p))
                .count() as u8,
            conflicts: verdict.cross_validation.conflicts_detected,
            formal_notation: verdict.formal_notation.clone(),
            #[cfg(feature = "serde")]
            trace_json: None, // Could serialize verdict.trace if desired.
            correlation_id: correlation_id.map(Into::into),
        });

        seq
    }

    /// Returns all entries since `after_seq` (exclusive).
    pub fn entries_since(&self, after_seq: u64) -> &[AuditEntry] {
        let start = self
            .entries
            .iter()
            .position(|e| e.seq > after_seq)
            .unwrap_or(self.entries.len());
        &self.entries[start..]
    }

    pub fn total_entries(&self) -> u64 {
        self.seq
    }
    pub fn permitted_count(&self) -> usize {
        self.entries.iter().filter(|e| e.permitted).count()
    }
    pub fn denied_count(&self) -> usize {
        self.entries.iter().filter(|e| !e.permitted).count()
    }

    /// Export all entries as NDJSON (one JSON object per line).
    #[cfg(feature = "serde")]
    pub fn to_ndjson(&self) -> String {
        self.entries
            .iter()
            .filter_map(|e| serde_json::to_string(e).ok())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}
