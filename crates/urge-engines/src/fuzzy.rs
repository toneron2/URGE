//! Fuzzy logic engine — reasoning with degrees of truth.
//!
//! Standard Zadeh fuzzy logic:
//!   - Conjunction (⊓): min(μ(a), μ(b))
//!   - Disjunction (⊔): max(μ(a), μ(b))
//!   - Negation:        1 - μ(a)
//!
//! A fact is its degree: a float or integer clamped to [0, 1], a boolean as 1 or 0. A fact
//! the context does not supply is 0 (closed world). Until 0.1.4 every variable read as 0.5,
//! so `a fuzzy_and b` held for every context.

use urge_core::{
    ast::{AstNode, Expr, Literal},
    decision::{Confidence, CrossValidation, EntryOutcome, LogicTrace, Stage, TraceEntry, Verdict},
    engine::{ContextValue, EngineError, EngineId, EvalContext, LogicEngine, Paradigm},
    symbol::{ParadigmSet, SemanticClass},
};

pub struct FuzzyEngine;

impl LogicEngine for FuzzyEngine {
    fn id(&self) -> EngineId {
        EngineId(5)
    }
    fn paradigm(&self) -> Paradigm {
        Paradigm::Fuzzy
    }
    fn name(&self) -> &'static str {
        "FuzzyEngine"
    }

    fn can_handle(&self, node: &AstNode) -> bool {
        matches!(
            node.as_ref(),
            Expr::Lit(Literal::Membership(_))
                | Expr::Lit(Literal::Probability(_))
                | Expr::Unary {
                    op: SemanticClass::MembershipDegree,
                    ..
                }
                | Expr::Binary {
                    op: SemanticClass::FuzzyAnd | SemanticClass::FuzzyOr,
                    ..
                }
        )
    }

    fn evaluate(&self, node: &AstNode, ctx: &EvalContext<'_>) -> Result<Verdict, EngineError> {
        let mut trace = LogicTrace::new();
        let mut paradigms = ParadigmSet::empty();
        paradigms.insert(Paradigm::Fuzzy);

        let degree = eval_fuzzy(node, ctx)?;
        // Fuzzy threshold: valid if degree >= 0.5 (midpoint of truth).
        let valid = degree >= 0.5;
        let confidence = Confidence((degree * 255.0) as u8);

        trace.push(TraceEntry {
            stage: Stage::EngineEvaluation,
            paradigm: Some(Paradigm::Fuzzy),
            description: if valid {
                "fuzzy: above threshold"
            } else {
                "fuzzy: below threshold"
            },
            outcome: if valid {
                EntryOutcome::Permitted
            } else {
                EntryOutcome::Denied
            },
        });

        Ok(Verdict {
            valid,
            confidence,
            paradigms_evaluated: paradigms,
            trace,
            cross_validation: CrossValidation::ok(),
            #[cfg(feature = "alloc")]
            formal_notation: alloc::format!("{} = {:.3}", fuzzy_notation(node), degree),
            #[cfg(feature = "alloc")]
            citations: alloc::vec![],
        })
    }
}

/// A fact's membership degree: a number clamped to [0, 1], a boolean as 1 or 0, anything
/// else (a string, an absent fact) 0.
fn degree_of(v: Option<&ContextValue>) -> f32 {
    match v {
        Some(ContextValue::Bool(b)) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Some(ContextValue::Integer(n)) => (*n as f32).clamp(0.0, 1.0),
        Some(ContextValue::Float(f)) => (*f as f32).clamp(0.0, 1.0),
        _ => 0.0,
    }
}

fn eval_fuzzy(node: &AstNode, ctx: &EvalContext<'_>) -> Result<f32, EngineError> {
    match node.as_ref() {
        Expr::Lit(Literal::Membership(m)) => Ok(m.clamp(0.0, 1.0)),
        Expr::Lit(Literal::Probability(p)) => Ok(p.clamp(0.0, 1.0)),
        Expr::Lit(Literal::Bool(b)) => Ok(if *b { 1.0 } else { 0.0 }),
        Expr::Lit(Literal::Integer(n)) => Ok((*n as f32).clamp(0.0, 1.0)),
        Expr::Lit(Literal::Float(f)) => Ok((*f as f32).clamp(0.0, 1.0)),
        Expr::Var { name, .. } => Ok(degree_of(
            ctx.slots
                .iter()
                .find(|(k, _)| *k == name.as_str())
                .map(|(_, v)| v),
        )),
        Expr::Binary {
            op: SemanticClass::FuzzyAnd,
            left,
            right,
            ..
        } => {
            let l = eval_fuzzy(left, ctx)?;
            let r = eval_fuzzy(right, ctx)?;
            Ok(l.min(r))
        }
        Expr::Binary {
            op: SemanticClass::FuzzyOr,
            left,
            right,
            ..
        } => {
            let l = eval_fuzzy(left, ctx)?;
            let r = eval_fuzzy(right, ctx)?;
            Ok(l.max(r))
        }
        Expr::Unary {
            op: SemanticClass::Negation,
            operand,
            ..
        } => {
            let v = eval_fuzzy(operand, ctx)?;
            Ok(1.0 - v)
        }
        Expr::Unary {
            op: SemanticClass::MembershipDegree,
            operand,
            ..
        } => eval_fuzzy(operand, ctx),
        // Anything else (an operator of another paradigm under a fuzzy connective) is not
        // this engine's to decide; it read as 0.5 until 0.1.4.
        _ => Err(EngineError::UnsupportedNode),
    }
}

#[cfg(feature = "alloc")]
fn fuzzy_notation(node: &AstNode) -> alloc::string::String {
    match node.as_ref() {
        Expr::Lit(Literal::Membership(m)) | Expr::Lit(Literal::Probability(m)) => {
            alloc::format!("{m}")
        }
        Expr::Lit(Literal::Bool(b)) => (if *b { "⊤" } else { "⊥" }).into(),
        Expr::Lit(Literal::Integer(n)) => alloc::format!("{n}"),
        Expr::Lit(Literal::Float(f)) => alloc::format!("{f}"),
        Expr::Var { name, .. } => name.as_str().into(),
        Expr::Binary {
            op: SemanticClass::FuzzyAnd,
            left,
            right,
            ..
        } => alloc::format!("({}) ⊓ ({})", fuzzy_notation(left), fuzzy_notation(right)),
        Expr::Binary {
            op: SemanticClass::FuzzyOr,
            left,
            right,
            ..
        } => alloc::format!("({}) ⊔ ({})", fuzzy_notation(left), fuzzy_notation(right)),
        Expr::Unary {
            op: SemanticClass::Negation,
            operand,
            ..
        } => alloc::format!("¬({})", fuzzy_notation(operand)),
        Expr::Unary {
            op: SemanticClass::MembershipDegree,
            operand,
            ..
        } => alloc::format!("μ({})", fuzzy_notation(operand)),
        _ => "…".into(),
    }
}
