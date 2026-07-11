//! Fuzzy logic engine — reasoning with degrees of truth.
//!
//! Standard Zadeh fuzzy logic:
//!   - Conjunction (⊓): min(μ(a), μ(b))
//!   - Disjunction (⊔): max(μ(a), μ(b))
//!   - Negation:        1 - μ(a)
//!
//! Useful in healthcare for "likely", "unlikely", "high probability" expressions.

use urge_core::{
    ast::{AstNode, Expr, Literal},
    decision::{Confidence, CrossValidation, EntryOutcome, LogicTrace, Stage, TraceEntry, Verdict},
    engine::{EngineError, EngineId, EvalContext, LogicEngine, Paradigm},
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
            formal_notation: alloc::format!("μ = {:.3}", degree),
            #[cfg(feature = "alloc")]
            citations: alloc::vec![],
        })
    }
}

fn eval_fuzzy(node: &AstNode, _ctx: &EvalContext<'_>) -> Result<f32, EngineError> {
    match node.as_ref() {
        Expr::Lit(Literal::Membership(m)) => Ok(m.clamp(0.0, 1.0)),
        Expr::Lit(Literal::Probability(p)) => Ok(p.clamp(0.0, 1.0)),
        Expr::Lit(Literal::Bool(b)) => Ok(if *b { 1.0 } else { 0.0 }),
        Expr::Binary {
            op: SemanticClass::FuzzyAnd,
            left,
            right,
            ..
        } => {
            let l = eval_fuzzy(left, _ctx)?;
            let r = eval_fuzzy(right, _ctx)?;
            Ok(l.min(r))
        }
        Expr::Binary {
            op: SemanticClass::FuzzyOr,
            left,
            right,
            ..
        } => {
            let l = eval_fuzzy(left, _ctx)?;
            let r = eval_fuzzy(right, _ctx)?;
            Ok(l.max(r))
        }
        Expr::Unary {
            op: SemanticClass::Negation,
            operand,
            ..
        } => {
            let v = eval_fuzzy(operand, _ctx)?;
            Ok(1.0 - v)
        }
        _ => Ok(0.5), // Unknown: maximally uncertain.
    }
}
