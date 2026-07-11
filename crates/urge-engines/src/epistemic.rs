//! Epistemic logic engine — reasoning about knowledge and belief.
//!
//! K(agent, φ): agent *knows* φ
//! B(agent, φ): agent *believes* φ
//! C(φ): common knowledge (all agents know φ)

use urge_core::{
    ast::{AstNode, Expr},
    decision::{Confidence, CrossValidation, EntryOutcome, LogicTrace, Stage, TraceEntry, Verdict},
    engine::{EngineError, EngineId, EvalContext, LogicEngine, Paradigm},
    symbol::{ParadigmSet, SemanticClass},
};

pub struct EpistemicEngine;

impl LogicEngine for EpistemicEngine {
    fn id(&self) -> EngineId {
        EngineId(2)
    }
    fn paradigm(&self) -> Paradigm {
        Paradigm::Epistemic
    }
    fn name(&self) -> &'static str {
        "EpistemicEngine"
    }

    fn can_handle(&self, node: &AstNode) -> bool {
        matches!(
            node.as_ref(),
            Expr::Apply {
                op: SemanticClass::Knows | SemanticClass::Believes | SemanticClass::CommonKnowledge,
                ..
            }
        )
    }

    fn evaluate(&self, node: &AstNode, ctx: &EvalContext<'_>) -> Result<Verdict, EngineError> {
        let mut trace = LogicTrace::new();
        let mut paradigms = ParadigmSet::empty();
        paradigms.insert(Paradigm::Epistemic);

        match node.as_ref() {
            Expr::Apply {
                op, agent, body, ..
            } => {
                // Context key: "knows:{agent}:{fact}" = true|false
                // or          "believes:{agent}:{fact}" = 0.0..1.0 (confidence)
                let body_result = crate::boolean::BooleanEngine.evaluate(body, ctx)?;

                let (valid, description) = match op {
                    SemanticClass::Knows => {
                        // K(a, φ): a knows φ iff φ is true AND a has access to that truth.
                        // Check for "knows:{agent}:*" = true in context.
                        #[cfg(feature = "alloc")]
                        let key = alloc::format!("knows:{}:granted", agent);
                        #[cfg(feature = "alloc")]
                        let knows = ctx
                            .slots
                            .iter()
                            .find(|(k, _)| *k == key.as_str())
                            .and_then(|(_, v)| v.as_bool())
                            .unwrap_or(body_result.valid);
                        #[cfg(not(feature = "alloc"))]
                        let knows = body_result.valid;

                        (
                            body_result.valid && knows,
                            if knows {
                                "K: agent knows and fact is true"
                            } else {
                                "K: agent lacks epistemic access"
                            },
                        )
                    }

                    SemanticClass::Believes => {
                        // B(a, φ): a believes φ even if φ is false (fallible belief).
                        #[cfg(feature = "alloc")]
                        let key = alloc::format!("believes:{}:granted", agent);
                        #[cfg(feature = "alloc")]
                        let believes = ctx
                            .slots
                            .iter()
                            .find(|(k, _)| *k == key.as_str())
                            .and_then(|(_, v)| v.as_bool())
                            .unwrap_or(true); // Default: believe what you see.
                        #[cfg(not(feature = "alloc"))]
                        let believes = true;

                        (
                            believes,
                            if believes {
                                "B: agent believes"
                            } else {
                                "B: agent does not believe"
                            },
                        )
                    }

                    SemanticClass::CommonKnowledge => {
                        // C(φ): all agents know φ.
                        let all_know = ctx
                            .slots
                            .iter()
                            .filter(|(k, _)| k.starts_with("agent:"))
                            .all(|(_, v)| v.as_bool().unwrap_or(false));
                        (
                            all_know && body_result.valid,
                            if all_know {
                                "C: common knowledge holds"
                            } else {
                                "C: not all agents know"
                            },
                        )
                    }

                    _ => return Err(EngineError::UnsupportedNode),
                };

                trace.push(TraceEntry {
                    stage: Stage::EngineEvaluation,
                    paradigm: Some(Paradigm::Epistemic),
                    description,
                    outcome: if valid {
                        EntryOutcome::Permitted
                    } else {
                        EntryOutcome::Denied
                    },
                });

                Ok(Verdict {
                    valid,
                    confidence: if valid {
                        Confidence::HIGH
                    } else {
                        Confidence::MEDIUM
                    },
                    paradigms_evaluated: paradigms,
                    trace,
                    cross_validation: CrossValidation::ok(),
                    #[cfg(feature = "alloc")]
                    formal_notation: alloc::format!(
                        "{}({}, {})",
                        match op {
                            SemanticClass::Knows => "K",
                            SemanticClass::Believes => "B",
                            SemanticClass::CommonKnowledge => "C",
                            _ => "?",
                        },
                        agent,
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
