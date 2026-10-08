//! Boolean propositional logic engine.
//!
//! This is the base paradigm — every expression passes through here first.
//! All other paradigms reduce to Boolean at the meta-engine synthesis stage.

use urge_core::{
    ast::{AstNode, Expr, Literal},
    decision::{Confidence, CrossValidation, EntryOutcome, LogicTrace, Stage, TraceEntry, Verdict},
    engine::{ContextValue, EngineError, EngineId, EvalContext, LogicEngine, Paradigm},
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
                        | SemanticClass::ExclusiveOr
                        | SemanticClass::Equals
                        | SemanticClass::NotEquals
                        | SemanticClass::LessThan
                        | SemanticClass::LessOrEqual
                        | SemanticClass::GreaterThan
                        | SemanticClass::GreaterOrEqual,
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
        Expr::Lit(Literal::Float(f)) => Ok(*f != 0.0),

        Expr::Var { name, .. } => {
            let key_str: &str = name.as_str();
            // Look up in context by iterating slots.
            for (k, v) in ctx.slots {
                if *k == key_str {
                    let b = truth(v);
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

        // A comparison of two numbers: facts, literals, or booleans as 1 and 0. A side the
        // context does not supply, or supplies as a string, makes the comparison false
        // (closed-world), and the trace says so.
        Expr::Binary {
            op, left, right, ..
        } if relational_symbol(*op).is_some() => {
            let (l, r) = (eval_num(left, ctx), eval_num(right, ctx));
            let (result, description) = match (l, r) {
                (Some(l), Some(r)) => {
                    let holds = match op {
                        SemanticClass::Equals => l == r,
                        SemanticClass::NotEquals => l != r,
                        SemanticClass::LessThan => l < r,
                        SemanticClass::LessOrEqual => l <= r,
                        SemanticClass::GreaterThan => l > r,
                        _ => l >= r,
                    };
                    (
                        holds,
                        if holds {
                            "comparison holds"
                        } else {
                            "comparison fails"
                        },
                    )
                }
                _ => (
                    false,
                    "comparison: a side is absent or not numeric (CWA: false)",
                ),
            };
            trace.push(TraceEntry {
                stage: Stage::EngineEvaluation,
                paradigm: Some(Paradigm::Boolean),
                description,
                outcome: if result {
                    EntryOutcome::Permitted
                } else {
                    EntryOutcome::Denied
                },
            });
            Ok(result)
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

/// A fact's truth in boolean position: a boolean is itself, a number is true when nonzero,
/// a string is false. Until 0.1.4 a number read as false.
fn truth(v: &ContextValue) -> bool {
    match v {
        ContextValue::Bool(b) => *b,
        ContextValue::Integer(n) => *n != 0,
        ContextValue::Float(f) => *f != 0.0,
        _ => false,
    }
}

/// A node's value in numeric position: a numeric literal, a boolean as 1 or 0, or a fact
/// supplied as a number or boolean. `None` for anything else, including an absent fact.
fn eval_num(node: &AstNode, ctx: &EvalContext<'_>) -> Option<f64> {
    match node.as_ref() {
        Expr::Lit(Literal::Bool(b)) => Some(if *b { 1.0 } else { 0.0 }),
        Expr::Lit(lit) => lit.as_f64(),
        Expr::Var { name, .. } => ctx
            .slots
            .iter()
            .find(|(k, _)| *k == name.as_str())
            .and_then(|(_, v)| match v {
                ContextValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
                ContextValue::Integer(n) => Some(*n as f64),
                ContextValue::Float(f) => Some(*f),
                _ => None,
            }),
        _ => None,
    }
}

/// The notation symbol of a relational operator, `None` for any other class.
fn relational_symbol(op: SemanticClass) -> Option<&'static str> {
    Some(match op {
        SemanticClass::Equals => "=",
        SemanticClass::NotEquals => "≠",
        SemanticClass::LessThan => "<",
        SemanticClass::LessOrEqual => "≤",
        SemanticClass::GreaterThan => ">",
        SemanticClass::GreaterOrEqual => "≥",
        _ => return None,
    })
}

#[cfg(feature = "alloc")]
fn format_notation(node: &AstNode) -> alloc::string::String {
    match node.as_ref() {
        Expr::Lit(Literal::Bool(b)) => (if *b { "⊤" } else { "⊥" }).into(),
        Expr::Lit(Literal::Integer(n)) => alloc::format!("{n}"),
        Expr::Lit(Literal::Float(f)) => alloc::format!("{f}"),
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
        Expr::Binary {
            op: SemanticClass::ExclusiveOr,
            left,
            right,
            ..
        } => alloc::format!("({}) ⊕ ({})", format_notation(left), format_notation(right)),
        Expr::Binary {
            op, left, right, ..
        } => match relational_symbol(*op) {
            Some(sym) => alloc::format!(
                "({}) {sym} ({})",
                format_notation(left),
                format_notation(right)
            ),
            None => alloc::string::String::from("…"),
        },
        _ => alloc::string::String::from("…"),
    }
}
