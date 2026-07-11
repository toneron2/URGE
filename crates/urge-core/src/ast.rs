//! Abstract Syntax Tree for multi-paradigm logical expressions.
//!
//! The AST is built by the tokenizer/parser stage of Figure 26.
//! Each node is annotated with the paradigm(s) it belongs to so the router
//! can dispatch without re-scanning the tree.
//!
//! ## Memory model
//!
//! - With `alloc`: nodes box their children (heap-allocated, unbounded depth).
//! - Without `alloc`: the `Shallow` variants use fixed-size inline children
//!   bounded by the `heapless` structures. This is suitable for embedded.

use crate::symbol::{ParadigmSet, SemanticClass};

// ── Literal values ─────────────────────────────────────────────────────────────

/// A literal value in an expression.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Literal {
    Bool(bool),
    Integer(i64),
    Float(f64),
    /// Short string that fits in 32 bytes without allocation.
    ShortStr(heapless::String<32>),
    #[cfg(feature = "alloc")]
    Str(alloc::string::String),
    /// Unix timestamp (nanoseconds) or monotonic counter for temporal engines.
    Time(u64),
    /// Probability in [0.0, 1.0].
    Probability(f32),
    /// Fuzzy membership degree in [0.0, 1.0].
    Membership(f32),
}

impl Literal {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Literal::Bool(b) => Some(*b),
            Literal::Integer(n) => Some(*n != 0),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Literal::Float(f) => Some(*f),
            Literal::Integer(n) => Some(*n as f64),
            Literal::Probability(p) => Some(*p as f64),
            Literal::Membership(m) => Some(*m as f64),
            _ => None,
        }
    }
}

// ── AST node ───────────────────────────────────────────────────────────────────

/// A node in the multi-paradigm AST.
///
/// The tree mixes paradigms freely: a Deontic `Obligatory` node can contain
/// a Temporal `Globally` subtree. The cross-validation stage checks that such
/// mixtures are coherent.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Expr {
    // ── Terminals ──────────────────────────────────────────────────────────
    Lit(Literal),
    Var {
        name: heapless::String<32>,
        paradigms: ParadigmSet,
    },

    // ── Unary operators ────────────────────────────────────────────────────
    Unary {
        op: SemanticClass,
        operand: AstNode,
        paradigms: ParadigmSet,
    },

    // ── Binary operators ───────────────────────────────────────────────────
    Binary {
        op: SemanticClass,
        left: AstNode,
        right: AstNode,
        paradigms: ParadigmSet,
    },

    // ── Ternary (e.g., temporal Until: φ U ψ) ─────────────────────────────
    Ternary {
        op: SemanticClass,
        first: AstNode,
        second: AstNode,
        third: AstNode,
        paradigms: ParadigmSet,
    },

    // ── Quantified formula ─────────────────────────────────────────────────
    Quantified {
        quantifier: SemanticClass,
        variable: heapless::String<32>,
        body: AstNode,
        paradigms: ParadigmSet,
    },

    // ── Application (agent · predicate in epistemic logic) ────────────────
    Apply {
        op: SemanticClass,
        agent: heapless::String<16>,
        body: AstNode,
        paradigms: ParadigmSet,
    },

    // ── Obligation / Permission / Prohibition with metadata ────────────────
    DeonticStatement {
        modality: SemanticClass, // Obligatory | Permitted | Forbidden
        agent: heapless::String<16>,
        action: heapless::String<32>,
        /// Optional deadline for obligation lifecycle tracking.
        deadline_ns: Option<u64>,
        /// Policy source citation (regulatory anchor).
        source: Option<heapless::String<64>>,
        paradigms: ParadigmSet,
    },

    // ── Temporal constraint ────────────────────────────────────────────────
    TemporalConstraint {
        op: SemanticClass, // Globally | Finally | Until | …
        body: AstNode,
        /// Absolute time bound in logical-time units.
        bound_ns: Option<u64>,
        paradigms: ParadigmSet,
    },
}

impl Expr {
    /// The set of paradigms this expression node participates in.
    pub fn paradigms(&self) -> ParadigmSet {
        match self {
            Expr::Lit(_) => ParadigmSet::empty(),
            Expr::Var { paradigms, .. } => *paradigms,
            Expr::Unary { paradigms, .. } => *paradigms,
            Expr::Binary { paradigms, .. } => *paradigms,
            Expr::Ternary { paradigms, .. } => *paradigms,
            Expr::Quantified { paradigms, .. } => *paradigms,
            Expr::Apply { paradigms, .. } => *paradigms,
            Expr::DeonticStatement { paradigms, .. } => *paradigms,
            Expr::TemporalConstraint { paradigms, .. } => *paradigms,
        }
    }
}

// ── Box alias for heap/no-heap compatibility ───────────────────────────────────

#[cfg(feature = "alloc")]
pub type AstNode = alloc::boxed::Box<Expr>;

/// Without `alloc` we can still represent leaf/single-level nodes inline.
/// Deep nesting requires the `alloc` feature.
#[cfg(not(feature = "alloc"))]
#[derive(Debug, Clone)]
pub struct AstNode(pub Expr);

#[cfg(feature = "alloc")]
pub fn node(expr: Expr) -> AstNode {
    alloc::boxed::Box::new(expr)
}

#[cfg(not(feature = "alloc"))]
pub fn node(expr: Expr) -> AstNode {
    AstNode(expr)
}

// ── Token (pre-AST) ────────────────────────────────────────────────────────────

/// A raw token produced by the tokenizer before AST construction.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// The semantic class resolved from the Unicode dictionary.
    pub class: SemanticClass,
    /// Raw text of the token for trace reconstruction.
    pub raw: heapless::String<32>,
    /// Character offset in the original input.
    pub offset: u32,
}

impl Token {
    pub fn new(class: SemanticClass, raw: &str, offset: u32) -> Self {
        let mut s = heapless::String::new();
        for ch in raw.chars().take(32) {
            let _ = s.push(ch);
        }
        Token {
            class,
            raw: s,
            offset,
        }
    }
}
