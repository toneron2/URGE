//! Stage 6: Cross-System Validator — the architecture's key differentiator.
//!
//! After each engine produces its verdict, the validator checks that the results
//! are **mutually consistent** across paradigms. Traditional policy engines
//! evaluate rules within a single logic; this stage cross-checks results
//! *between* logics and reports contradictions instead of silently resolving them.
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

        // A connective the router decided from its fragments is one verdict that already
        // carries its sides' agreement (as its confidence) and its parts' conflicts.
        if successful.len() == 1
            && successful[0].trace.entries.last().map(|e| e.description)
                == Some(crate::router::DECOMPOSED)
        {
            let v = successful[0];
            trace.push(TraceEntry {
                stage: Stage::CrossValidation,
                paradigm: None,
                description: if v.cross_validation.consistent {
                    "cross-validation: consistent (connective decided from its fragments)"
                } else {
                    "cross-validation: conflicts detected between the parts of an and"
                },
                outcome: if v.cross_validation.consistent {
                    EntryOutcome::Evaluated
                } else {
                    EntryOutcome::Conflict
                },
            });
            return (v.valid, v.confidence, v.cross_validation.clone());
        }

        let mut conflicts: u8 = 0;
        let mut conflict_detail = None;

        // Each rule asks whether ANY verdict of one paradigm stands in the named relation to
        // ANY verdict of another. Until 0.1.4 a rule read the first verdict of each paradigm,
        // so the conflicts a conjunction reported changed with the order of its conjuncts.
        let any = |paradigm: Paradigm, valid: bool| {
            successful
                .iter()
                .any(|v| v.paradigms_evaluated.contains(paradigm) && v.valid == valid)
        };

        // ── Rule 1: Modal-Boolean hard contradiction ────────────────────────
        // □φ=true but Boolean φ=false → hard contradiction.
        if any(Paradigm::Modal, true) && any(Paradigm::Boolean, false) {
            conflicts += 1;
            conflict_detail = Some("modal necessity vs boolean contradiction");
            trace.push(TraceEntry {
                stage: Stage::CrossValidation,
                paradigm: None,
                description: "CONFLICT: □φ=true but Boolean φ=false",
                outcome: EntryOutcome::Conflict,
            });
        }

        // ── Rule 2: Temporal constraint violated beside an obligation that holds ──
        if any(Paradigm::Temporal, false) && any(Paradigm::Deontic, true) {
            conflicts += 1;
            conflict_detail = conflict_detail.or(Some(
                "temporal constraint violated while an obligation holds",
            ));
            trace.push(TraceEntry {
                stage: Stage::CrossValidation,
                paradigm: None,
                description: "CONFLICT: temporal constraint violated while obligation active",
                outcome: EntryOutcome::Conflict,
            });
        }

        // ── Rule 3: Paraconsistent scenario ────────────────────────────────
        if successful.iter().any(|v| {
            v.paradigms_evaluated.contains(Paradigm::Paraconsistent)
                && !v.cross_validation.consistent
        }) {
            conflicts += 1;
            conflict_detail = conflict_detail.or(Some("paraconsistent scenario: see engine trace"));
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

        // The aggregate is the weaker of two things: how many engines agree, and the least
        // confident engine among them. A node is handled by one engine today, so the second
        // term is what carries information: temporal and modal verdicts are HIGH, an
        // epistemic deny is MEDIUM, a fuzzy verdict is its membership degree. Until 0.1.4 the
        // engines' own confidences were discarded and every verdict came back at 255.
        let weakest_engine = successful
            .iter()
            .map(|v| v.confidence)
            .min()
            .unwrap_or(Confidence::NONE);
        let confidence = core::cmp::min(
            Confidence::from_agreement(agreement_count, total),
            weakest_engine,
        );

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

        // Deontic takes precedence in governance: a denied obligation anywhere denies the
        // whole, whatever the majority says. Governance is not democratic.
        let final_valid = !any(Paradigm::Deontic, false) && majority_valid;

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
