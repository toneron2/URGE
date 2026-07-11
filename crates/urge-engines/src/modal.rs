//! Modal logic engine (S5 axiom system).
//!
//! Handles necessity (□) and possibility (◇) operators.
//! In the governance context: □φ means "φ is invariant across all accessible
//! worlds" (i.e., regardless of system state). ◇φ means "φ is achievable".

use urge_core::{
    ast::{AstNode, Expr},
    decision::{Confidence, CrossValidation, EntryOutcome, LogicTrace, Stage, TraceEntry, Verdict},
    engine::{EngineError, EngineId, EvalContext, LogicEngine, Paradigm},
    symbol::{ParadigmSet, SemanticClass},
};

pub struct ModalEngine;

impl LogicEngine for ModalEngine {
    fn id(&self) -> EngineId {
        EngineId(1)
    }
    fn paradigm(&self) -> Paradigm {
        Paradigm::Modal
    }
    fn name(&self) -> &'static str {
        "ModalEngine"
    }

    fn can_handle(&self, node: &AstNode) -> bool {
        matches!(
            node.as_ref(),
            Expr::Unary {
                op: SemanticClass::Necessity | SemanticClass::Possibility,
                ..
            }
        )
    }

    fn evaluate(&self, node: &AstNode, ctx: &EvalContext<'_>) -> Result<Verdict, EngineError> {
        let mut trace = LogicTrace::new();
        let mut paradigms = ParadigmSet::empty();
        paradigms.insert(Paradigm::Modal);

        match node.as_ref() {
            Expr::Unary { op, operand, .. } => {
                let body = crate::boolean::BooleanEngine.evaluate(operand, ctx)?;

                let (valid, notation) = match op {
                    SemanticClass::Necessity => {
                        // □φ: necessarily true — check it holds in current context
                        // AND that no context variable contradicts it.
                        let contradicted = ctx.slots.iter().any(|(k, v)| {
                            k.starts_with("world:") && v.as_bool().is_some_and(|b| !b)
                        });
                        let v = body.valid && !contradicted;
                        trace.push(TraceEntry {
                            stage: Stage::EngineEvaluation,
                            paradigm: Some(Paradigm::Modal),
                            description: if v {
                                "□φ: necessary and holds"
                            } else {
                                "□φ: contradicted in some world"
                            },
                            outcome: if v {
                                EntryOutcome::Permitted
                            } else {
                                EntryOutcome::Denied
                            },
                        });
                        #[cfg(feature = "alloc")]
                        let note = alloc::format!("□({})", body.formal_notation);
                        #[cfg(not(feature = "alloc"))]
                        let note = ();
                        (v, note)
                    }

                    SemanticClass::Possibility => {
                        // ◇φ: possibly true — true if body holds in ANY accessible world.
                        // In our single-world context: true if body.valid OR if there exists
                        // a "world:possible:{key}" = true in context.
                        let possible_in_context = ctx.slots.iter().any(|(k, v)| {
                            k.starts_with("world:possible:") && v.as_bool().unwrap_or(false)
                        });
                        let v = body.valid || possible_in_context;
                        trace.push(TraceEntry {
                            stage: Stage::EngineEvaluation,
                            paradigm: Some(Paradigm::Modal),
                            description: if v {
                                "◇φ: possible"
                            } else {
                                "◇φ: impossible in all worlds"
                            },
                            outcome: if v {
                                EntryOutcome::Permitted
                            } else {
                                EntryOutcome::Denied
                            },
                        });
                        #[cfg(feature = "alloc")]
                        let note = alloc::format!("◇({})", body.formal_notation);
                        #[cfg(not(feature = "alloc"))]
                        let note = ();
                        (v, note)
                    }

                    _ => return Err(EngineError::UnsupportedNode),
                };

                Ok(Verdict {
                    valid,
                    confidence: Confidence::HIGH,
                    paradigms_evaluated: paradigms,
                    trace,
                    cross_validation: CrossValidation::ok(),
                    #[cfg(feature = "alloc")]
                    formal_notation: notation,
                    #[cfg(feature = "alloc")]
                    citations: alloc::vec![],
                })
            }
            _ => Err(EngineError::UnsupportedNode),
        }
    }
}
