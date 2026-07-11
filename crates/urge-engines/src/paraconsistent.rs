//! Paraconsistent logic engine.
//!
//! Handles contradictions without explosion (ex contradictione quodlibet).
//! When deontic obligations conflict, the paraconsistent engine flags the
//! contradiction and computes a "most permissible" resolution rather than
//! producing ⊥ (false) for everything.
//!
//! Uses the Belnap four-valued logic: TRUE, FALSE, BOTH, NEITHER.

use urge_core::{
    ast::{AstNode, Expr},
    decision::{Confidence, CrossValidation, EntryOutcome, LogicTrace, Stage, TraceEntry, Verdict},
    engine::{EngineError, EngineId, EvalContext, LogicEngine, Paradigm},
    symbol::{ParadigmSet, SemanticClass},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BelnapValue {
    // Part of the complete Belnap 4-valued set. This engine handles only the
    // contradictory/gappy cases, so plain `True` is never constructed here, but
    // it is matched in the outcome mapping below for completeness.
    #[allow(dead_code)]
    True,
    False,
    Both,
    Neither,
}

pub struct ParaconsistentEngine;

impl LogicEngine for ParaconsistentEngine {
    fn id(&self) -> EngineId {
        EngineId(7)
    }
    fn paradigm(&self) -> Paradigm {
        Paradigm::Paraconsistent
    }
    fn name(&self) -> &'static str {
        "ParaconsistentEngine"
    }

    fn can_handle(&self, node: &AstNode) -> bool {
        matches!(
            node.as_ref(),
            Expr::Unary {
                op: SemanticClass::BothTrueAndFalse | SemanticClass::NeitherTrueNorFalse,
                ..
            }
        )
    }

    fn evaluate(&self, node: &AstNode, ctx: &EvalContext<'_>) -> Result<Verdict, EngineError> {
        let mut trace = LogicTrace::new();
        let mut paradigms = ParadigmSet::empty();
        paradigms.insert(Paradigm::Paraconsistent);

        let belnap = match node.as_ref() {
            Expr::Unary {
                op: SemanticClass::BothTrueAndFalse,
                operand,
                ..
            } => {
                let body = crate::boolean::BooleanEngine.evaluate(operand, ctx)?;
                if body.valid {
                    BelnapValue::Both // Marked as contradictory but true.
                } else {
                    BelnapValue::False
                }
            }
            Expr::Unary {
                op: SemanticClass::NeitherTrueNorFalse,
                ..
            } => BelnapValue::Neither,
            _ => return Err(EngineError::UnsupportedNode),
        };

        let (valid, description) = match belnap {
            BelnapValue::True => (true, "paraconsistent: TRUE"),
            BelnapValue::False => (false, "paraconsistent: FALSE"),
            BelnapValue::Both => (
                true,
                "paraconsistent: BOTH (contradiction, using true-leaning resolution)",
            ),
            BelnapValue::Neither => (
                false,
                "paraconsistent: NEITHER (gap, using false-leaning resolution)",
            ),
        };

        let consistent = !matches!(belnap, BelnapValue::Both | BelnapValue::Neither);
        trace.push(TraceEntry {
            stage: Stage::EngineEvaluation,
            paradigm: Some(Paradigm::Paraconsistent),
            description,
            outcome: if valid {
                EntryOutcome::Permitted
            } else {
                EntryOutcome::Denied
            },
        });

        Ok(Verdict {
            valid,
            confidence: if consistent {
                Confidence::CERTAIN
            } else {
                Confidence::LOW
            },
            paradigms_evaluated: paradigms,
            trace,
            cross_validation: CrossValidation {
                consistent,
                conflicts_detected: if consistent { 0 } else { 1 },
                conflict_detail: if consistent {
                    None
                } else {
                    Some("paraconsistent scenario detected")
                },
            },
            #[cfg(feature = "alloc")]
            formal_notation: alloc::format!("Belnap::{:?}", belnap),
            #[cfg(feature = "alloc")]
            citations: alloc::vec![],
        })
    }
}
