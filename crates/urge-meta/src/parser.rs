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
//! prefix    ::= IDENTIFIER | LITERAL | '(' expr ')'
//! ```

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
}

#[cfg(feature = "alloc")]
impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> Option<AstNode> {
        self.parse_expr(0)
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
        let token = self.peek()?.clone();

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
                let val: i64 = token.raw.as_str().parse().unwrap_or(0);
                node(Expr::Lit(Literal::Integer(val)))
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

            SemanticClass::Necessity | SemanticClass::Possibility => {
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
                let op = token.class;
                self.consume();
                let body = self.parse_expr(20)?;
                let mut ps = ParadigmSet::empty();
                ps.insert(urge_core::engine::Paradigm::Temporal);
                node(Expr::TemporalConstraint {
                    op,
                    body,
                    bound_ns: None,
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

            _ => {
                self.consume();
                node(Expr::Lit(Literal::Bool(false)))
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
