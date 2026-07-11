//! Boolean propositional logic engine.
//!
//! This is the base paradigm — every expression passes through here first.
//! All other paradigms reduce to Boolean at the meta-engine synthesis stage.

use urge_core::{
    ast::{AstNode, Expr, Literal},
    decision::{Confidence, CrossValidation, EntryOutcome, LogicTrace, Stage, TraceEntry, Verdict},
    engine::{EngineError, EngineId, EvalContext, LogicEngine, Paradigm},
    symbol::{ParadigmSet, SemanticClass},
};

pub struct BooleanEngine;

impl LogicEngine for BooleanEngine {
    fn id(&self) -> EngineId {
        EngineId(0)
    }
    fn paradigm(&self) -> Paradigm {
        Paradigm::Boolean
    }
    fn name(&self) -> &'static str {
        "BooleanEngine"
    }

    fn can_handle(&self, node: &AstNode) -> bool {
        matches!(
            node.as_ref(),
            Expr::Lit(_)
                | Expr::Var { .. }
                | Expr::Unary {
                    op: SemanticClass::Negation,
                    ..
                }
                | Expr::Binary {
                    op: SemanticClass::Conjunction
                        | SemanticClass::Disjunction
                        | SemanticClass::Implication
                        | SemanticClass::Biconditional
                        | SemanticClass::ExclusiveOr,
                    ..
                }
        )
    }

    fn evaluate(&self, node: &AstNode, ctx: &EvalContext<'_>) -> Result<Verdict, EngineError> {
        let mut trace = LogicTrace::new();
        let result = eval_bool(node, ctx, &mut trace, 0)?;

        let mut paradigms = ParadigmSet::empty();
        paradigms.insert(Paradigm::Boolean);

        Ok(Verdict {
            valid: result,
            confidence: Confidence::CERTAIN,
            paradigms_evaluated: paradigms,
            trace,
            cross_validation: CrossValidation::ok(),
            #[cfg(feature = "alloc")]
            formal_notation: format_notation(node),
            #[cfg(feature = "alloc")]
            citations: alloc::vec![],
        })
    }
}

fn eval_bool(
    node: &AstNode,
    ctx: &EvalContext<'_>,
    trace: &mut LogicTrace,
    depth: u8,
) -> Result<bool, EngineError> {
    if depth > ctx.depth_limit {
        return Err(EngineError::DepthLimitExceeded);
    }

    match node.as_ref() {
        Expr::Lit(Literal::Bool(b)) => {
            trace.push(TraceEntry {
                stage: Stage::EngineEvaluation,
                paradigm: Some(Paradigm::Boolean),
                description: "literal",
                outcome: if *b {
                    EntryOutcome::Permitted
                } else {
                    EntryOutcome::Denied
                },
            });
            Ok(*b)
        }

        Expr::Lit(Literal::Integer(n)) => Ok(*n != 0),

        Expr::Var { name, .. } => {
            let key_str: &str = name.as_str();
            // Look up in context by iterating slots.
            for (k, v) in ctx.slots {
                if *k == key_str {
                    let b = v.as_bool().unwrap_or(false);
                    trace.push(TraceEntry {
                        stage: Stage::EngineEvaluation,
                        paradigm: Some(Paradigm::Boolean),
                        description: "variable lookup",
                        outcome: if b {
                            EntryOutcome::Permitted
                        } else {
                            EntryOutcome::Denied
                        },
                    });
                    return Ok(b);
                }
            }
            // Variable not found — treat as false (closed-world assumption).
            trace.push(TraceEntry {
                stage: Stage::EngineEvaluation,
                paradigm: Some(Paradigm::Boolean),
                description: "variable not found (CWA: false)",
                outcome: EntryOutcome::Denied,
            });
            Ok(false)
        }

        Expr::Unary {
            op: SemanticClass::Negation,
            operand,
            ..
        } => {
            let inner = eval_bool(operand, ctx, trace, depth + 1)?;
            Ok(!inner)
        }

        Expr::Binary {
            op, left, right, ..
        } => {
            let l = eval_bool(left, ctx, trace, depth + 1)?;
            match op {
                SemanticClass::Conjunction => {
                    // Short-circuit: don't evaluate right if left is false.
                    if !l {
                        return Ok(false);
                    }
                    let r = eval_bool(right, ctx, trace, depth + 1)?;
                    Ok(l && r)
                }
                SemanticClass::Disjunction => {
                    if l {
                        return Ok(true);
                    }
                    let r = eval_bool(right, ctx, trace, depth + 1)?;
                    Ok(l || r)
                }
                SemanticClass::Implication => {
                    let r = eval_bool(right, ctx, trace, depth + 1)?;
                    Ok(!l || r)
                }
                SemanticClass::Biconditional => {
                    let r = eval_bool(right, ctx, trace, depth + 1)?;
                    Ok(l == r)
                }
                SemanticClass::ExclusiveOr => {
                    let r = eval_bool(right, ctx, trace, depth + 1)?;
                    Ok(l ^ r)
                }
                _ => Err(EngineError::UnsupportedNode),
            }
        }

        _ => Err(EngineError::UnsupportedNode),
    }
}

#[cfg(feature = "alloc")]
fn format_notation(node: &AstNode) -> alloc::string::String {
    match node.as_ref() {
        Expr::Lit(Literal::Bool(b)) => (if *b { "⊤" } else { "⊥" }).to_string(),
        Expr::Lit(Literal::Integer(n)) => alloc::format!("{n}"),
        Expr::Var { name, .. } => name.as_str().into(),
        Expr::Unary {
            op: SemanticClass::Negation,
            operand,
            ..
        } => alloc::format!("¬({})", format_notation(operand)),
        Expr::Binary {
            op: SemanticClass::Conjunction,
            left,
            right,
            ..
        } => alloc::format!("({}) ∧ ({})", format_notation(left), format_notation(right)),
        Expr::Binary {
            op: SemanticClass::Disjunction,
            left,
            right,
            ..
        } => alloc::format!("({}) ∨ ({})", format_notation(left), format_notation(right)),
        Expr::Binary {
            op: SemanticClass::Implication,
            left,
            right,
            ..
        } => alloc::format!("({}) → ({})", format_notation(left), format_notation(right)),
        Expr::Binary {
            op: SemanticClass::Biconditional,
            left,
            right,
            ..
        } => alloc::format!("({}) ↔ ({})", format_notation(left), format_notation(right)),
        _ => alloc::string::String::from("…"),
    }
}
