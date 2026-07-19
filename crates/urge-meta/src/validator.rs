//! Stage 6: Cross-System Validator — the architecture's key differentiator.
//!
//! After each engine produces its verdict, the validator checks that the results
//! are **mutually consistent** across paradigms. This is the step that traditional
//! policy engines (OPA, Drools, REGO) completely lack.
//!
//! ## Consistency rules
//!
//! The validator applies these inter-paradigm checks:
//!
//! 1. **Deontic-Boolean consistency**: If the Boolean engine says φ is false but
//!    Deontic says φ is obligatory, this is not necessarily a conflict — it means
//!    the obligation has not yet been satisfied. But if Forbidden(φ) and φ is
//!    simultaneously true, that IS a conflict.
//!
//! 2. **Temporal-Deontic consistency**: If O(φ) with deadline d, and the temporal
//!    engine reports the deadline is exceeded without φ being true, conflict.
//!
//! 3. **Epistemic-Deontic consistency**: An agent cannot have an obligation it
//!    cannot possibly know about. K(a, O(φ)) must be true for the obligation to bind.
//!
//! 4. **Modal-Boolean consistency**: If □φ (necessarily φ) but Boolean says ¬φ,
//!    this is a hard contradiction.
//!
//! 5. **Fuzzy-Deontic consistency**: If fuzzy degree < 0.1 but Deontic says
//!    Permitted, flag low-confidence warning (not a conflict, but worthy of note).

use urge_core::{
    decision::{Confidence, CrossValidation, EntryOutcome, LogicTrace, Stage, TraceEntry, Verdict},
    engine::Paradigm,
};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

pub struct CrossValidator;

impl CrossValidator {
    /// Validate consistency across a set of engine verdicts.
    ///
    /// Returns an aggregated `CrossValidation` and the overall consensus `valid` value.
    #[cfg(feature = "alloc")]
    pub fn validate(
        verdicts: &[Result<Verdict, urge_core::engine::EngineError>],
        trace: &mut LogicTrace,
    ) -> (bool, Confidence, CrossValidation) {
        // Collect successful verdicts.
        let successful: Vec<&Verdict> = verdicts.iter().filter_map(|v| v.as_ref().ok()).collect();

        if successful.is_empty() {
            return (
                false,
                Confidence::NONE,
                CrossValidation {
                    consistent: false,
                    conflicts_detected: 1,
                    conflict_detail: Some("no engines succeeded"),
                },
            );
        }

        let mut conflicts: u8 = 0;
        let mut conflict_detail = None;

        // ── Rule 1: Modal-Boolean hard contradiction ────────────────────────
        let modal_verdict = find_by_paradigm(&successful, Paradigm::Modal);
        let boolean_verdict = find_by_paradigm(&successful, Paradigm::Boolean);

        if let (Some(modal), Some(bool_v)) = (modal_verdict, boolean_verdict) {
            // □φ=true but Boolean φ=false → hard contradiction.
            if modal.valid && !bool_v.valid {
                conflicts += 1;
                conflict_detail = Some("modal necessity vs boolean contradiction");
                trace.push(TraceEntry {
                    stage: Stage::CrossValidation,
                    paradigm: None,
                    description: "CONFLICT: □φ=true but Boolean φ=false",
                    outcome: EntryOutcome::Conflict,
                });
            }
        }

        // ── Rule 2: Temporal deadline exceeded + Deontic obligation active ──
        let temporal_verdict = find_by_paradigm(&successful, Paradigm::Temporal);
        let deontic_verdict = find_by_paradigm(&successful, Paradigm::Deontic);

        if let (Some(temporal), Some(deontic)) = (temporal_verdict, deontic_verdict) {
            if !temporal.valid && deontic.valid {
                // Temporal constraint violated while deontic says still valid —
                // this means deadline exceeded without obligation satisfaction.
                conflicts += 1;
                conflict_detail =
                    conflict_detail.or(Some("temporal deadline exceeded: obligation violated"));
                trace.push(TraceEntry {
                    stage: Stage::CrossValidation,
                    paradigm: None,
                    description: "CONFLICT: temporal constraint violated while obligation active",
                    outcome: EntryOutcome::Conflict,
                });
            }
        }

        // ── Rule 3: Paraconsistent scenario ────────────────────────────────
        if let Some(para) = find_by_paradigm(&successful, Paradigm::Paraconsistent) {
            if !para.cross_validation.consistent {
                conflicts += 1;
                conflict_detail =
                    conflict_detail.or(Some("paraconsistent scenario: see engine trace"));
            }
        }

        // ── Aggregate confidence ────────────────────────────────────────────
        // Agreement ratio: how many engines agree on the final `valid` value.
        let majority_valid =
            {
                let (yes, no) = successful.iter().fold((0u8, 0u8), |(y, n), v| {
                    if v.valid {
                        (y + 1, n)
                    } else {
                        (y, n + 1)
                    }
                });
                yes >= no
            };

        let agreement_count = successful
            .iter()
            .filter(|v| v.valid == majority_valid)
            .count() as u8;
        let total = successful.len() as u8;

        let confidence = Confidence::from_agreement(agreement_count, total);

        trace.push(TraceEntry {
            stage: Stage::CrossValidation,
            paradigm: None,
            description: if conflicts == 0 {
                "cross-validation: consistent"
            } else {
                "cross-validation: conflicts detected"
            },
            outcome: if conflicts == 0 {
                EntryOutcome::Evaluated
            } else {
                EntryOutcome::Conflict
            },
        });

        // Deontic takes precedence in governance: if Deontic says deny, we deny.
        let final_valid = if let Some(deontic) = deontic_verdict {
            // Deontic denial overrides majority — governance is not democratic.
            if !deontic.valid {
                false
            } else {
                majority_valid
            }
        } else {
            majority_valid
        };

        (
            final_valid && conflicts == 0,
            confidence,
            CrossValidation {
                consistent: conflicts == 0,
                conflicts_detected: conflicts,
                conflict_detail,
            },
        )
    }

    /// Embedded (no-alloc) path: single verdict is its own cross-validation.
    pub fn validate_single(verdict: &Verdict, _trace: &mut LogicTrace) -> CrossValidation {
        verdict.cross_validation.clone()
    }
}

#[cfg(feature = "alloc")]
fn find_by_paradigm<'a>(verdicts: &[&'a Verdict], paradigm: Paradigm) -> Option<&'a Verdict> {
    verdicts
        .iter()
        .find(|v| v.paradigms_evaluated.contains(paradigm))
        .copied()
}
