//! Temporal / LTL logic engine.
//!
//! Implements Linear Temporal Logic operators: G (globally/always), F (finally/
//! eventually), X (next), U (until), R (release).
//!
//! ## Evaluation model
//!
//! Full LTL model-checking requires a trace (sequence of states). For the
//! governance use case, we operate in two modes:
//!
//! 1. **Constraint mode**: The expression specifies a constraint on the current
//!    logical time. E.g., `G(consent_obtained)` means "always must be true" and
//!    is checked against current context.
//!
//! 2. **Monitoring mode**: Used by `urge-monitor` for continuous evaluation over
//!    an event stream. This engine handles the *instantaneous* check; the
//!    monitor drives the time loop.

use urge_core::{
    ast::{AstNode, Expr},
    decision::{Confidence, CrossValidation, EntryOutcome, LogicTrace, Stage, TraceEntry, Verdict},
    engine::{EngineError, EngineId, EvalContext, LogicEngine, Paradigm},
    symbol::{ParadigmSet, SemanticClass},
};

pub struct TemporalEngine;

impl LogicEngine for TemporalEngine {
    fn id(&self) -> EngineId {
        EngineId(4)
    }
    fn paradigm(&self) -> Paradigm {
        Paradigm::Temporal
    }
    fn name(&self) -> &'static str {
        "TemporalEngine"
    }

    fn can_handle(&self, node: &AstNode) -> bool {
        matches!(
            node.as_ref(),
            Expr::TemporalConstraint { .. }
                | Expr::Unary {
                    op: SemanticClass::Globally | SemanticClass::Finally | SemanticClass::Next,
                    ..
                }
                | Expr::Binary {
                    op: SemanticClass::Until | SemanticClass::Release | SemanticClass::WeakUntil,
                    ..
                }
        )
    }

    fn evaluate(&self, node: &AstNode, ctx: &EvalContext<'_>) -> Result<Verdict, EngineError> {
        let mut trace = LogicTrace::new();
        let mut paradigms = ParadigmSet::empty();
        paradigms.insert(Paradigm::Temporal);

        let (valid, notation) = eval_temporal(node, ctx, &mut trace, 0)?;

        Ok(Verdict {
            valid,
            confidence: Confidence::HIGH, // Temporal is inherently probabilistic about future states.
            paradigms_evaluated: paradigms,
            trace,
            cross_validation: CrossValidation::ok(),
            #[cfg(feature = "alloc")]
            formal_notation: notation,
            #[cfg(feature = "alloc")]
            citations: alloc::vec![],
        })
    }
}

#[cfg(feature = "alloc")]
fn eval_temporal(
    node: &AstNode,
    ctx: &EvalContext<'_>,
    trace: &mut LogicTrace,
    depth: u8,
) -> Result<(bool, alloc::string::String), EngineError> {
    if depth > ctx.depth_limit {
        return Err(EngineError::DepthLimitExceeded);
    }

    match node.as_ref() {
        Expr::TemporalConstraint {
            op, body, bound_ns, ..
        } => {
            let now = ctx.logical_time;

            match op {
                SemanticClass::Globally => {
                    // G(φ): φ must hold at current time.
                    // We evaluate the body as a current-state check.
                    let body_result = crate::boolean::BooleanEngine.evaluate(body, ctx)?;
                    let valid = body_result.valid;
                    trace.push(TraceEntry {
                        stage: Stage::EngineEvaluation,
                        paradigm: Some(Paradigm::Temporal),
                        description: if valid {
                            "G(φ): currently holds"
                        } else {
                            "G(φ): currently violated"
                        },
                        outcome: if valid {
                            EntryOutcome::Permitted
                        } else {
                            EntryOutcome::Denied
                        },
                    });
                    Ok((valid, alloc::format!("G({})", body_result.formal_notation)))
                }

                SemanticClass::Finally => {
                    // F(φ) with bound: must happen by deadline.
                    // Without bound: optimistically true (future is open).
                    let within_deadline = bound_ns.is_none_or(|d| now <= d);
                    let body_result = crate::boolean::BooleanEngine.evaluate(body, ctx)?;
                    let valid = body_result.valid || within_deadline;
                    trace.push(TraceEntry {
                        stage: Stage::EngineEvaluation,
                        paradigm: Some(Paradigm::Temporal),
                        description: if body_result.valid {
                            "F(φ): already satisfied"
                        } else if within_deadline {
                            "F(φ): deadline not yet reached"
                        } else {
                            "F(φ): deadline exceeded without satisfaction"
                        },
                        outcome: if valid {
                            EntryOutcome::Permitted
                        } else {
                            EntryOutcome::Denied
                        },
                    });
                    Ok((valid, alloc::format!("F({})", body_result.formal_notation)))
                }

                SemanticClass::Next => {
                    // X(φ): In governance, treat "next tick" as "immediately after this action".
                    // Mark as pending — monitor will evaluate at next event.
                    trace.push(TraceEntry {
                        stage: Stage::EngineEvaluation,
                        paradigm: Some(Paradigm::Temporal),
                        description: "X(φ): deferred to next evaluation cycle",
                        outcome: EntryOutcome::Evaluated,
                    });
                    Ok((true, "X(…)".to_string()))
                }

                _ => Err(EngineError::UnsupportedNode),
            }
        }

        Expr::Binary {
            op: SemanticClass::Until,
            left,
            right,
            ..
        } => {
            // φ U ψ: φ must hold until ψ becomes true.
            let phi = crate::boolean::BooleanEngine.evaluate(left, ctx)?;
            let psi = crate::boolean::BooleanEngine.evaluate(right, ctx)?;

            let valid = psi.valid || phi.valid;
            trace.push(TraceEntry {
                stage: Stage::EngineEvaluation,
                paradigm: Some(Paradigm::Temporal),
                description: if psi.valid {
                    "U: right-hand side satisfied"
                } else if phi.valid {
                    "U: left-hand holds, waiting for right"
                } else {
                    "U: both sides false — until violated"
                },
                outcome: if valid {
                    EntryOutcome::Permitted
                } else {
                    EntryOutcome::Denied
                },
            });
            Ok((
                valid,
                alloc::format!("({}) U ({})", phi.formal_notation, psi.formal_notation),
            ))
        }

        _ => {
            // Delegate non-temporal nodes to Boolean.
            let r = crate::boolean::BooleanEngine.evaluate(node, ctx)?;
            Ok((r.valid, r.formal_notation))
        }
    }
}

#[cfg(not(feature = "alloc"))]
fn eval_temporal(
    node: &AstNode,
    ctx: &EvalContext<'_>,
    trace: &mut LogicTrace,
    depth: u8,
) -> Result<(bool, ()), EngineError> {
    // Simplified no-alloc path: only handle TemporalConstraint directly.
    match node.as_ref() {
        Expr::TemporalConstraint { op, bound_ns, .. } => {
            let now = ctx.logical_time;
            let within = bound_ns.map_or(true, |d| now <= d);
            trace.push(TraceEntry {
                stage: Stage::EngineEvaluation,
                paradigm: Some(Paradigm::Temporal),
                description: if within {
                    "temporal: within bound"
                } else {
                    "temporal: exceeded bound"
                },
                outcome: if within {
                    EntryOutcome::Permitted
                } else {
                    EntryOutcome::Denied
                },
            });
            Ok((within, ()))
        }
        _ => Ok((true, ())),
    }
}
