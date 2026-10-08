//! Why a verdict came out as it did.
//!
//! [`GovernancePipeline::explain`] walks the expression's top-level tree of boolean
//! connectives (`and`, `or`, `implies`, `iff`, `xor`) and evaluates each clause under it on
//! its own, through the same pipeline and configuration, recording the facts each clause
//! read. `deciding` names the clauses that decided a deny: the false sides of an `and`,
//! every side of a false `or`, the true antecedent and false consequent of a false
//! `implies`, and both sides of a false `iff` or `xor`.
//!
//! A fact the context does not supply reads as false; its `value` is `None` so a caller can
//! tell "false" from "never supplied".

use alloc::{string::String, string::ToString, vec::Vec};

use urge_core::{
    ast::{AstNode, Expr},
    decision::LogicTrace,
    engine::{ContextValue, EvalContext, Paradigm},
    symbol::{ParadigmSet, SemanticClass},
};

use crate::{parser::Parser, pipeline::GovernancePipeline, tokenizer::Tokenizer};

/// One fact a clause read, as the context supplied it.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Fact {
    pub name: String,
    /// The supplied value, rendered; `None` when the context has no such slot.
    pub value: Option<String>,
}

/// One clause under the expression's connective tree, evaluated on its own.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Clause {
    pub notation: String,
    pub valid: bool,
    pub facts: Vec<Fact>,
}

/// The clauses of an expression and the ones that decided a deny.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Explanation {
    /// The connective tree's truth from its clauses. The pipeline's verdict can still deny
    /// it on a cross-paradigm conflict or the confidence threshold.
    pub valid: bool,
    pub clauses: Vec<Clause>,
    /// Indices into `clauses`; empty when `valid`.
    pub deciding: Vec<usize>,
    /// Why the expression did not parse, when it did not.
    pub error: Option<String>,
}

fn connective(op: SemanticClass) -> bool {
    matches!(
        op,
        SemanticClass::Conjunction
            | SemanticClass::Disjunction
            | SemanticClass::Implication
            | SemanticClass::Biconditional
            | SemanticClass::ExclusiveOr
    )
}

fn render(v: &ContextValue) -> String {
    match v {
        ContextValue::Bool(b) => b.to_string(),
        ContextValue::Integer(n) => n.to_string(),
        ContextValue::Float(f) => f.to_string(),
        ContextValue::Str(s) => alloc::format!("{s:?}"),
        ContextValue::OwnedStr(s) => alloc::format!("{s:?}"),
    }
}

fn facts(node: &AstNode, ctx: &EvalContext<'_>, out: &mut Vec<Fact>) {
    let mut add = |name: String| {
        if !out.iter().any(|f| f.name == name) {
            let value = ctx
                .slots
                .iter()
                .find(|(k, _)| *k == name.as_str())
                .map(|(_, v)| render(v));
            out.push(Fact { name, value });
        }
    };
    match node.as_ref() {
        Expr::Var { name, .. } => add(name.as_str().to_string()),
        Expr::Unary { operand, .. } => facts(operand, ctx, out),
        Expr::Binary { left, right, .. } => {
            facts(left, ctx, out);
            facts(right, ctx, out);
        }
        Expr::TemporalConstraint { body, .. } => facts(body, ctx, out),
        Expr::Apply {
            op, agent, body, ..
        } => {
            match op {
                SemanticClass::Knows => add(alloc::format!("knows:{agent}:granted")),
                SemanticClass::Believes => add(alloc::format!("believes:{agent}:granted")),
                _ => {}
            }
            facts(body, ctx, out);
        }
        _ => {}
    }
}

impl GovernancePipeline {
    /// The clauses of `expression` against `ctx`, each with its truth and the facts it read,
    /// and the clauses that decided a deny.
    pub fn explain(&self, expression: &str, ctx: &EvalContext<'_>) -> Explanation {
        let mut parser = Parser::new(Tokenizer::new().tokenize(expression));
        let ast = match parser.parse() {
            Some(a) => a,
            None => {
                return Explanation {
                    valid: false,
                    clauses: Vec::new(),
                    deciding: Vec::new(),
                    error: Some(
                        parser
                            .error
                            .unwrap_or_else(|| String::from("empty expression")),
                    ),
                }
            }
        };
        let mut clauses = Vec::new();
        let (valid, deciding) = self.walk(&ast, ctx, &mut clauses);
        Explanation {
            valid,
            clauses,
            deciding: if valid { Vec::new() } else { deciding },
            error: None,
        }
    }

    /// (truth, the clauses that make it false) for one node of the connective tree.
    fn walk(
        &self,
        node: &AstNode,
        ctx: &EvalContext<'_>,
        clauses: &mut Vec<Clause>,
    ) -> (bool, Vec<usize>) {
        if let Expr::Binary {
            op, left, right, ..
        } = node.as_ref()
        {
            if connective(*op) {
                let start = clauses.len();
                let (lv, ld) = self.walk(left, ctx, clauses);
                let mid = clauses.len();
                let (rv, rd) = self.walk(right, ctx, clauses);
                let end = clauses.len();
                let both = || (start..end).collect::<Vec<usize>>();
                return match op {
                    SemanticClass::Conjunction => {
                        let mut d = Vec::new();
                        if !lv {
                            d.extend(ld);
                        }
                        if !rv {
                            d.extend(rd);
                        }
                        (lv && rv, d)
                    }
                    SemanticClass::Disjunction => (lv || rv, ld.into_iter().chain(rd).collect()),
                    SemanticClass::Implication => (!lv || rv, (start..mid).chain(rd).collect()),
                    SemanticClass::Biconditional => (lv == rv, both()),
                    _ => (lv ^ rv, both()),
                };
            }
        }
        let mut paradigms = ParadigmSet::empty();
        if self.config.exhaustive_evaluation {
            for &p in Paradigm::ALL {
                paradigms.insert(p);
            }
        } else {
            paradigms = node.paradigms();
            paradigms.insert(Paradigm::Boolean);
        }
        let v = self.evaluate_ast(node, paradigms, ctx, LogicTrace::new());
        let mut fs = Vec::new();
        facts(node, ctx, &mut fs);
        clauses.push(Clause {
            notation: v.formal_notation.clone(),
            valid: v.valid,
            facts: fs,
        });
        let i = clauses.len() - 1;
        (v.valid, if v.valid { Vec::new() } else { alloc::vec![i] })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PipelineConfig;

    fn ctx(slots: &'static [(&'static str, ContextValue)]) -> EvalContext<'static> {
        EvalContext {
            slots,
            logical_time: 0,
            depth_limit: 16,
        }
    }

    #[test]
    fn the_false_conjuncts_decide() {
        static S: [(&str, ContextValue); 2] = [
            ("a", ContextValue::Bool(true)),
            ("b", ContextValue::Bool(false)),
        ];
        let e = GovernancePipeline::new(PipelineConfig::default())
            .explain("must a and must b and must_not c", &ctx(&S));
        assert!(!e.valid);
        assert_eq!(e.clauses.len(), 3);
        assert_eq!(e.deciding, alloc::vec![1]);
        assert_eq!(e.clauses[1].notation, "O(b)");
        assert_eq!(
            e.clauses[2].facts[0],
            Fact {
                name: "c".into(),
                value: None
            },
            "c was never supplied"
        );
        assert!(e.clauses[2].valid, "must_not c permits when c reads false");
    }

    #[test]
    fn a_false_implication_names_its_antecedent_and_consequent() {
        static S: [(&str, ContextValue); 2] = [
            ("a", ContextValue::Bool(true)),
            ("b", ContextValue::Bool(false)),
        ];
        let e = GovernancePipeline::new(PipelineConfig::default())
            .explain("must a implies always b", &ctx(&S));
        assert!(!e.valid);
        assert_eq!(e.deciding, alloc::vec![0, 1]);
    }

    #[test]
    fn a_parse_error_is_explained() {
        let e = GovernancePipeline::new(PipelineConfig::default()).explain("a b", &ctx(&[]));
        assert!(e.error.unwrap().contains("unparsed input from 'b'"));
    }
}
