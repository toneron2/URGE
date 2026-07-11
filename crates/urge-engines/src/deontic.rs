//! Deontic logic engine — the governance heart of URGE.
//!
//! Deontic logic reasons about obligations, permissions, and prohibitions.
//! This engine is the primary path for all governance decisions: "must",
//! "may", "must not", "ought to", "forbidden", "permitted".
//!
//! ## SDL (Standard Deontic Logic) basis
//!
//! The engine implements SDL axioms:
//!   - O(φ) → P(φ)         (what is obligatory is permitted)
//!   - ¬(O(φ) ∧ O(¬φ))    (no contradictory obligations)
//!   - O(φ ∧ ψ) ↔ O(φ) ∧ O(ψ) (ought-conjunction)
//!   - F(φ) ↔ O(¬φ)        (forbidden is obligatory-not)
//!
//! ## Obligation lifecycle
//!
//! Each `DeonticStatement` can carry a `deadline_ns`. The monitor crate tracks
//! these over time. This engine only evaluates the *instantaneous* deontic status.

use urge_core::{
    ast::{AstNode, Expr},
    decision::{Confidence, CrossValidation, EntryOutcome, LogicTrace, Stage, TraceEntry, Verdict},
    engine::{EngineError, EngineId, EvalContext, LogicEngine, Paradigm},
    symbol::{ParadigmSet, SemanticClass},
};

pub struct DeonticEngine;

impl LogicEngine for DeonticEngine {
    fn id(&self) -> EngineId {
        EngineId(3)
    }
    fn paradigm(&self) -> Paradigm {
        Paradigm::Deontic
    }
    fn name(&self) -> &'static str {
        "DeonticEngine"
    }

    fn can_handle(&self, node: &AstNode) -> bool {
        matches!(
            node.as_ref(),
            Expr::DeonticStatement { .. }
                | Expr::Unary {
                    op: SemanticClass::Obligatory
                        | SemanticClass::Permitted
                        | SemanticClass::Forbidden,
                    ..
                }
        )
    }

    fn evaluate(&self, node: &AstNode, ctx: &EvalContext<'_>) -> Result<Verdict, EngineError> {
        let mut trace = LogicTrace::new();
        let mut paradigms = ParadigmSet::empty();
        paradigms.insert(Paradigm::Deontic);

        match node.as_ref() {
            Expr::DeonticStatement {
                modality,
                agent,
                action,
                deadline_ns,
                source,
                ..
            } => {
                // Check context for explicit permission grants/denials first.
                // Context key pattern: "deontic:{agent}:{action}" = "permitted"|"forbidden"|"obligatory"
                #[cfg(feature = "alloc")]
                let ctx_key_owned = alloc::format!("deontic:{}:{}", agent, action);
                #[cfg(feature = "alloc")]
                let ctx_key: &str = &ctx_key_owned;
                #[cfg(not(feature = "alloc"))]
                let ctx_key: &str = "deontic:__lookup_unsupported__";

                // Try to find a context override.
                let ctx_override = ctx
                    .slots
                    .iter()
                    .find(|(k, _)| *k == ctx_key)
                    .map(|(_, v)| v);

                let (valid, description, outcome) = match modality {
                    SemanticClass::Obligatory => {
                        // O(action): agent MUST perform action.
                        // Check if action has been performed (context key "{agent}:{action}:done").
                        #[cfg(feature = "alloc")]
                        let done_key = alloc::format!("{}:{}:done", agent, action);
                        #[cfg(feature = "alloc")]
                        let done = ctx
                            .slots
                            .iter()
                            .find(|(k, _)| *k == done_key.as_str())
                            .and_then(|(_, v)| v.as_bool())
                            .unwrap_or(false);
                        #[cfg(not(feature = "alloc"))]
                        let done = false;

                        // Check deadline if present.
                        let expired = deadline_ns.is_some_and(|d| ctx.logical_time > d);
                        if expired && !done {
                            (
                                false,
                                "obligation violated: deadline exceeded without performance",
                                EntryOutcome::Denied,
                            )
                        } else if done {
                            (true, "obligation satisfied", EntryOutcome::Permitted)
                        } else {
                            (
                                true,
                                "obligation active, deadline not exceeded",
                                EntryOutcome::Evaluated,
                            )
                        }
                    }

                    SemanticClass::Forbidden => {
                        // F(action) = O(¬action): agent MUST NOT perform action.
                        // If action is being evaluated as about-to-happen, deny.
                        #[cfg(feature = "alloc")]
                        let attempting_key = alloc::format!("{}:{}:attempting", agent, action);
                        #[cfg(feature = "alloc")]
                        let attempting = ctx
                            .slots
                            .iter()
                            .find(|(k, _)| *k == attempting_key.as_str())
                            .and_then(|(_, v)| v.as_bool())
                            .unwrap_or(false);
                        #[cfg(not(feature = "alloc"))]
                        let attempting = false;

                        if attempting {
                            (
                                false,
                                "forbidden action attempted — denied",
                                EntryOutcome::Denied,
                            )
                        } else {
                            (
                                true,
                                "forbidden action not attempted",
                                EntryOutcome::Permitted,
                            )
                        }
                    }

                    SemanticClass::Permitted => {
                        // P(action): action is permitted unless explicitly forbidden.
                        // Apply closed-permission assumption: check if there is a
                        // contradicting O(¬action) in context.
                        let denied_by_context = ctx_override
                            .and_then(|v| v.as_bool())
                            .map(|b| !b)
                            .unwrap_or(false);
                        if denied_by_context {
                            (
                                false,
                                "permission overridden by context prohibition",
                                EntryOutcome::Denied,
                            )
                        } else {
                            (true, "action permitted", EntryOutcome::Permitted)
                        }
                    }

                    _ => return Err(EngineError::MalformedExpression),
                };

                trace.push(TraceEntry {
                    stage: Stage::EngineEvaluation,
                    paradigm: Some(Paradigm::Deontic),
                    description,
                    outcome,
                });

                // Emit source citation if present.
                if let Some(src) = source {
                    trace.push(TraceEntry {
                        stage: Stage::EngineEvaluation,
                        paradigm: Some(Paradigm::Deontic),
                        description: "policy source cited",
                        outcome: EntryOutcome::Evaluated,
                    });
                    let _ = src; // Source stored in formal_notation below.
                }

                Ok(Verdict {
                    valid,
                    confidence: Confidence::CERTAIN,
                    paradigms_evaluated: paradigms,
                    trace,
                    cross_validation: CrossValidation::ok(),
                    #[cfg(feature = "alloc")]
                    formal_notation: alloc::format!(
                        "{}({}) for agent={} [{}]",
                        deontic_symbol(*modality),
                        action,
                        agent,
                        source.as_deref().unwrap_or("no-source")
                    ),
                    #[cfg(feature = "alloc")]
                    citations: alloc::vec![],
                })
            }

            Expr::Unary { op, operand, .. } => {
                // Wrap raw unary deontic and delegate body to boolean.
                trace.push(TraceEntry {
                    stage: Stage::EngineRouting,
                    paradigm: Some(Paradigm::Deontic),
                    description: "unary deontic operator",
                    outcome: EntryOutcome::Routed,
                });
                // Evaluate the body as Boolean (inner expression).
                let body_result = crate::boolean::BooleanEngine.evaluate(operand, ctx)?;
                let valid = match op {
                    SemanticClass::Obligatory => body_result.valid,
                    SemanticClass::Permitted => body_result.valid,
                    SemanticClass::Forbidden => !body_result.valid,
                    _ => return Err(EngineError::UnsupportedNode),
                };
                Ok(Verdict {
                    valid,
                    confidence: body_result.confidence,
                    paradigms_evaluated: paradigms,
                    trace,
                    cross_validation: CrossValidation::ok(),
                    #[cfg(feature = "alloc")]
                    formal_notation: alloc::format!(
                        "{}({})",
                        deontic_symbol(*op),
                        body_result.formal_notation
                    ),
                    #[cfg(feature = "alloc")]
                    citations: alloc::vec![],
                })
            }

            _ => Err(EngineError::UnsupportedNode),
        }
    }
}

fn deontic_symbol(cls: SemanticClass) -> &'static str {
    match cls {
        SemanticClass::Obligatory => "O",
        SemanticClass::Permitted => "P",
        SemanticClass::Forbidden => "F",
        SemanticClass::Waived => "W",
        _ => "?",
    }
}
