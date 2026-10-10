//! Stage 3: AST construction from token stream.
//!
//! Implements a simple Pratt (top-down operator precedence) parser.
//! The grammar covers all operators in the Unicode Semantic Dictionary.
//!
//! ## Grammar (informal)
//!
//! ```text
//! expr      ::= prefix unary_op? binary_rhs*
//! unary_op  ::= NOT | NECESSITY | POSSIBILITY | GLOBALLY | FINALLY | NEXT
//!             | OBLIGATORY | PERMITTED | FORBIDDEN
//! binary_rhs::= binary_op expr
//! binary_op ::= AND | OR | IMPLIES | IFF | XOR | UNTIL | RELEASE
//!             | EQ | NEQ | LT | LTE | GT | GTE
//! prefix    ::= IDENTIFIER | LITERAL
//!             | (GLOBALLY | FINALLY) expr bound?    -- G≤N, F≤N
//!             | NEXT expr
//!             | NEVER expr bound?                    -- G(¬expr)
//!             | (KNOWS | BELIEVES) IDENTIFIER expr   -- K(agent, expr), B(agent, expr)
//!             | COMMON_KNOWLEDGE expr
//! bound     ::= (WITHIN | BEFORE | DEADLINE) NUMBER  -- in EvalContext::logical_time units
//! ```
//!
//! Parentheses are not part of the grammar: the tokenizer skips them, so a prefix
//! operator applies to the next operand only (`must a or b` is `O(a) ∨ b`).
//!
//! An expression must parse completely. An operator with no supported reading here, or
//! input left over after the expression, is a parse error with a reason in
//! [`Parser::error`]; the pipeline denies it and names the reason.

use urge_core::{
    ast::{node, AstNode, Expr, Literal, Token},
    symbol::{ParadigmSet, SemanticClass},
};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Pratt parser: converts a token stream into an AST.
#[cfg(feature = "alloc")]
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// Why the last [`Parser::parse`] returned `None`.
    pub error: Option<alloc::string::String>,
}

#[cfg(feature = "alloc")]
impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            pos: 0,
            error: None,
        }
    }

    /// Parse the whole token stream. `None` when it does not parse completely; the
    /// reason is in [`Parser::error`].
    pub fn parse(&mut self) -> Option<AstNode> {
        let ast = self.parse_expr(0)?;
        if let Some(t) = self.peek().cloned() {
            return self.fail(alloc::format!(
                "unparsed input from '{}' at offset {}",
                t.raw,
                t.offset
            ));
        }
        Some(ast)
    }

    fn fail(&mut self, reason: alloc::string::String) -> Option<AstNode> {
        if self.error.is_none() {
            self.error = Some(reason);
        }
        None
    }

    /// A bound after a temporal operand: `within N`, `before N` or `deadline N`.
    fn bound(&mut self) -> Option<u64> {
        let t = self.peek()?;
        if !matches!(t.raw.as_str(), "within" | "before" | "deadline") {
            return None;
        }
        let n = self.tokens.get(self.pos + 1)?;
        if n.class != SemanticClass::NumericLiteral {
            return None;
        }
        let v: f64 = n.raw.as_str().parse().ok()?;
        if v.is_nan() || v < 0.0 {
            return None;
        }
        self.pos += 2;
        Some(v as u64)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn consume(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let t = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(t)
        } else {
            None
        }
    }

    /// Binding power (precedence) for binary operators.
    fn infix_bp(class: SemanticClass) -> Option<(u8, u8)> {
        match class {
            SemanticClass::Biconditional => Some((1, 2)),
            SemanticClass::Implication => Some((3, 4)),
            SemanticClass::Disjunction | SemanticClass::FuzzyOr => Some((5, 6)),
            SemanticClass::Conjunction | SemanticClass::FuzzyAnd => Some((7, 8)),
            SemanticClass::Until | SemanticClass::Release | SemanticClass::WeakUntil => {
                Some((9, 10))
            }
            SemanticClass::Equals
            | SemanticClass::NotEquals
            | SemanticClass::LessThan
            | SemanticClass::LessOrEqual
            | SemanticClass::GreaterThan
            | SemanticClass::GreaterOrEqual => Some((11, 12)),
            SemanticClass::ExclusiveOr => Some((13, 14)),
            _ => None,
        }
    }

    fn parse_expr(&mut self, min_bp: u8) -> Option<AstNode> {
        // ── Prefix / atom ──────────────────────────────────────────────────
        let token = match self.peek() {
            Some(t) => t.clone(),
            None => {
                return self.fail(alloc::string::String::from(
                    "the expression ends where an operand is expected",
                ))
            }
        };

        let mut lhs = match token.class {
            // Literals
            SemanticClass::Verum => {
                self.consume();
                node(Expr::Lit(Literal::Bool(true)))
            }
            SemanticClass::Falsum => {
                self.consume();
                node(Expr::Lit(Literal::Bool(false)))
            }
            SemanticClass::NumericLiteral => {
                self.consume();
                let raw = token.raw.as_str();
                // `2.5` is a float; until 0.1.4 it parsed as the integer 0.
                let lit = if raw.contains('.') {
                    Literal::Float(raw.parse().unwrap_or(0.0))
                } else {
                    Literal::Integer(raw.parse().unwrap_or(0))
                };
                node(Expr::Lit(lit))
            }
            SemanticClass::BooleanLiteral => {
                self.consume();
                let b = token.raw.as_str() == "true";
                node(Expr::Lit(Literal::Bool(b)))
            }

            // Unary operators
            SemanticClass::Negation => {
                self.consume();
                let operand = self.parse_expr(20)?;
                let mut ps = ParadigmSet::empty();
                for &p in SemanticClass::Negation.paradigms() {
                    ps.insert(p);
                }
                node(Expr::Unary {
                    op: SemanticClass::Negation,
                    operand,
                    paradigms: ps,
                })
            }

            // Modal □ ◇, fuzzy `mu φ`, and the Belnap markers `both φ` / `neither φ`: one
            // operand each, routed by the operator's own paradigm.
            SemanticClass::Necessity
            | SemanticClass::Possibility
            | SemanticClass::MembershipDegree
            | SemanticClass::BothTrueAndFalse
            | SemanticClass::NeitherTrueNorFalse => {
                let op = token.class;
                self.consume();
                let operand = self.parse_expr(20)?;
                let mut ps = ParadigmSet::empty();
                for &p in op.paradigms() {
                    ps.insert(p);
                }
                node(Expr::Unary {
                    op,
                    operand,
                    paradigms: ps,
                })
            }

            SemanticClass::Globally | SemanticClass::Finally | SemanticClass::Next => {
                // `never φ` is the dictionary's Globally(¬): G(¬φ), not G(φ).
                let never = token.raw.as_str() == "never";
                let op = token.class;
                self.consume();
                let mut body = self.parse_expr(20)?;
                if never {
                    let mut neg = ParadigmSet::empty();
                    for &p in SemanticClass::Negation.paradigms() {
                        neg.insert(p);
                    }
                    body = node(Expr::Unary {
                        op: SemanticClass::Negation,
                        operand: body,
                        paradigms: neg,
                    });
                }
                // A bound belongs to `always`, `never` and `eventually`; after `next` it is
                // left unparsed, which is an error rather than a bound silently ignored.
                let bound_ns = if op == SemanticClass::Next {
                    None
                } else {
                    self.bound()
                };
                let mut ps = ParadigmSet::empty();
                ps.insert(urge_core::engine::Paradigm::Temporal);
                node(Expr::TemporalConstraint {
                    op,
                    body,
                    bound_ns,
                    paradigms: ps,
                })
            }

            // Epistemic: `knows agent φ`, `believes agent φ`, `common_knowledge φ`.
            SemanticClass::Knows | SemanticClass::Believes | SemanticClass::CommonKnowledge => {
                let op = token.class;
                self.consume();
                let mut agent = heapless::String::new();
                if op != SemanticClass::CommonKnowledge {
                    match self.peek().cloned() {
                        Some(a) if a.class == SemanticClass::Identifier => {
                            self.consume();
                            for c in a.raw.chars().take(16) {
                                let _ = agent.push(c);
                            }
                        }
                        _ => {
                            return self.fail(alloc::format!(
                                "'{}' needs an agent: {} <agent> <claim>",
                                token.raw,
                                token.raw
                            ))
                        }
                    }
                }
                let body = self.parse_expr(20)?;
                let mut ps = ParadigmSet::empty();
                ps.insert(urge_core::engine::Paradigm::Epistemic);
                node(Expr::Apply {
                    op,
                    agent,
                    body,
                    paradigms: ps,
                })
            }

            SemanticClass::Obligatory | SemanticClass::Permitted | SemanticClass::Forbidden => {
                let modality = token.class;
                self.consume();
                // Expect: OBLIGATORY '(' agent ',' action ')'
                // Simplified: parse body expression as the action.
                let body = self.parse_expr(20)?;
                let mut ps = ParadigmSet::empty();
                ps.insert(urge_core::engine::Paradigm::Deontic);
                node(Expr::Unary {
                    op: modality,
                    operand: body,
                    paradigms: ps,
                })
            }

            // Identifier
            SemanticClass::Identifier => {
                self.consume();
                let mut ps = ParadigmSet::empty();
                ps.insert(urge_core::engine::Paradigm::Boolean);
                let mut name = heapless::String::new();
                for c in token.raw.chars().take(32) {
                    let _ = name.push(c);
                }
                node(Expr::Var {
                    name,
                    paradigms: ps,
                })
            }

            // `likely`, `unlikely`, `probability`: classified, but no engine exists yet.
            SemanticClass::Probability => {
                return self.fail(alloc::format!(
                    "'{}' at offset {}: probabilistic operators have no engine yet",
                    token.raw,
                    token.offset
                ))
            }

            // Anything else in operand position (a binary operator, a string, an operator
            // with no reading here such as distributed_knowledge) is an error, never the
            // literal false it used to become.
            _ => {
                return self.fail(alloc::format!(
                    "'{}' at offset {} cannot start an operand",
                    token.raw,
                    token.offset
                ))
            }
        };

        // ── Binary operators ───────────────────────────────────────────────
        while let Some(op_token) = self.peek().cloned() {
            if let Some((l_bp, r_bp)) = Self::infix_bp(op_token.class) {
                if l_bp < min_bp {
                    break;
                }
                self.consume();

                // Temporal Until/Release: binary temporal
                if matches!(
                    op_token.class,
                    SemanticClass::Until | SemanticClass::Release | SemanticClass::WeakUntil
                ) {
                    let right = self.parse_expr(r_bp)?;
                    let mut ps = ParadigmSet::empty();
                    ps.insert(urge_core::engine::Paradigm::Temporal);
                    lhs = node(Expr::Binary {
                        op: op_token.class,
                        left: lhs,
                        right,
                        paradigms: ps,
                    });
                } else {
                    let right = self.parse_expr(r_bp)?;
                    let mut ps = lhs.paradigms();
                    for &p in op_token.class.paradigms() {
                        ps.insert(p);
                    }
                    lhs = node(Expr::Binary {
                        op: op_token.class,
                        left: lhs,
                        right,
                        paradigms: ps,
                    });
                }
            } else {
                break;
            }
        }

        Some(lhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::Tokenizer;

    #[test]
    #[cfg(feature = "alloc")]
    fn parse_simple_conjunction() {
        let t = Tokenizer::new();
        let tokens = t.tokenize("a and b");
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        assert!(ast.is_some());
        assert!(matches!(
            ast.unwrap().as_ref(),
            Expr::Binary {
                op: SemanticClass::Conjunction,
                ..
            }
        ));
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn parse_deontic_obligation() {
        let t = Tokenizer::new();
        let tokens = t.tokenize("must obtain_consent");
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        assert!(ast.is_some());
        assert!(matches!(
            ast.unwrap().as_ref(),
            Expr::Unary {
                op: SemanticClass::Obligatory,
                ..
            }
        ));
    }

    fn parse_str(s: &str) -> (Option<AstNode>, Option<alloc::string::String>) {
        let mut p = Parser::new(Tokenizer::new().tokenize(s));
        let a = p.parse();
        (a, p.error)
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn never_is_globally_not() {
        let (ast, _) = parse_str("never breach");
        match ast.unwrap().as_ref() {
            Expr::TemporalConstraint {
                op: SemanticClass::Globally,
                body,
                ..
            } => {
                assert!(matches!(
                    body.as_ref(),
                    Expr::Unary {
                        op: SemanticClass::Negation,
                        ..
                    }
                ))
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn fuzzy_and_belnap_prefixes_parse() {
        for (src, op) in [
            ("mu risk", SemanticClass::MembershipDegree),
            ("both sensor_ok", SemanticClass::BothTrueAndFalse),
            ("neither sensor_ok", SemanticClass::NeitherTrueNorFalse),
        ] {
            let (ast, err) = parse_str(src);
            assert!(
                matches!(ast.as_deref(), Some(Expr::Unary { op: o, .. }) if *o == op),
                "{src}: {err:?}"
            );
        }
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn knows_takes_an_agent() {
        let (ast, _) = parse_str("knows nurse consent_given");
        assert!(
            matches!(ast.unwrap().as_ref(), Expr::Apply { op: SemanticClass::Knows, agent, .. } if agent.as_str() == "nurse")
        );
        let (ast, err) = parse_str("knows");
        assert!(ast.is_none() && err.unwrap().contains("needs an agent"));
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn a_bound_follows_a_temporal_operand() {
        let (ast, _) = parse_str("eventually reply within 30");
        assert!(matches!(
            ast.unwrap().as_ref(),
            Expr::TemporalConstraint {
                op: SemanticClass::Finally,
                bound_ns: Some(30),
                ..
            }
        ));
        let (ast, _) = parse_str("always heartbeat deadline 10");
        assert!(matches!(
            ast.unwrap().as_ref(),
            Expr::TemporalConstraint {
                op: SemanticClass::Globally,
                bound_ns: Some(10),
                ..
            }
        ));
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn leftover_input_and_unknown_operands_are_errors() {
        let (ast, err) = parse_str("agent must obtain_consent");
        assert!(ast.is_none() && err.unwrap().contains("unparsed input from 'must'"));
        let (ast, err) = parse_str("distributed_knowledge x");
        assert!(ast.is_none() && err.unwrap().contains("cannot start an operand"));
        let (ast, err) = parse_str("must");
        assert!(ast.is_none() && err.unwrap().contains("ends where an operand"));
        let (ast, err) = parse_str("likely rain");
        assert!(ast.is_none() && err.unwrap().contains("probabilistic operators"));
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn a_decimal_literal_is_a_float() {
        let (ast, _) = parse_str("2.5");
        assert!(matches!(
            ast.unwrap().as_ref(),
            Expr::Lit(Literal::Float(f)) if *f == 2.5
        ));
        let (ast, _) = parse_str("25");
        assert!(matches!(
            ast.unwrap().as_ref(),
            Expr::Lit(Literal::Integer(25))
        ));
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn a_prefix_operator_binds_its_operand_only() {
        let (ast, _) = parse_str("must a or b");
        assert!(matches!(
            ast.unwrap().as_ref(),
            Expr::Binary {
                op: SemanticClass::Disjunction,
                ..
            }
        ));
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn parse_temporal_globally() {
        let t = Tokenizer::new();
        let tokens = t.tokenize("always consent_valid");
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        assert!(ast.is_some());
        assert!(matches!(
            ast.unwrap().as_ref(),
            Expr::TemporalConstraint {
                op: SemanticClass::Globally,
                ..
            }
        ));
    }
}
