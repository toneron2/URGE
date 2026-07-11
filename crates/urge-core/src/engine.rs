//! Logic engine trait and paradigm enumeration.
//!
//! Every logic paradigm in the system is represented by a [`Paradigm`] variant
//! and implemented by a type that satisfies [`LogicEngine`]. The meta-engine
//! (Figure 26) uses the engine registry to route AST nodes to the correct
//! evaluator at runtime without allocation.

use crate::ast::AstNode;
use crate::decision::Verdict;

/// All supported logic paradigms, ordered for stable bit-mapping.
///
/// The order here defines the bit positions in [`crate::symbol::ParadigmSet`].
/// **Do not reorder without a semver bump.**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum Paradigm {
    Boolean = 0,
    Modal = 1,
    Epistemic = 2,
    Deontic = 3,
    Temporal = 4,
    Fuzzy = 5,
    Probabilistic = 6,
    Paraconsistent = 7,
    // Room for up to 8 more within a u16 ParadigmSet
}

impl Paradigm {
    pub const ALL: &'static [Paradigm] = &[
        Paradigm::Boolean,
        Paradigm::Modal,
        Paradigm::Epistemic,
        Paradigm::Deontic,
        Paradigm::Temporal,
        Paradigm::Fuzzy,
        Paradigm::Probabilistic,
        Paradigm::Paraconsistent,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Paradigm::Boolean => "boolean",
            Paradigm::Modal => "modal",
            Paradigm::Epistemic => "epistemic",
            Paradigm::Deontic => "deontic",
            Paradigm::Temporal => "temporal",
            Paradigm::Fuzzy => "fuzzy",
            Paradigm::Probabilistic => "probabilistic",
            Paradigm::Paraconsistent => "paraconsistent",
        }
    }
}

/// Stable integer identifier for an engine instance.
/// Used in logic traces so the trace is self-describing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineId(pub u8);

/// Errors that a logic engine can return.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    /// The AST node type is not supported by this engine.
    UnsupportedNode,
    /// A required context value is absent.
    MissingContext(&'static str),
    /// The expression is syntactically malformed for this paradigm.
    MalformedExpression,
    /// The engine encountered a contradiction it cannot resolve.
    ContradictionDetected,
    /// Evaluation exceeded the allowed recursion depth.
    DepthLimitExceeded,
}

impl EngineError {
    pub fn as_str(&self) -> &'static str {
        match self {
            EngineError::UnsupportedNode => "unsupported_node",
            EngineError::MissingContext(_) => "missing_context",
            EngineError::MalformedExpression => "malformed_expression",
            EngineError::ContradictionDetected => "contradiction_detected",
            EngineError::DepthLimitExceeded => "depth_limit_exceeded",
        }
    }
}

#[cfg(feature = "std")]
impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::UnsupportedNode => write!(f, "UnsupportedNode"),
            EngineError::MissingContext(key) => write!(f, "MissingContext({key})"),
            EngineError::MalformedExpression => write!(f, "MalformedExpression"),
            EngineError::ContradictionDetected => write!(f, "ContradictionDetected"),
            EngineError::DepthLimitExceeded => write!(f, "DepthLimitExceeded"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EngineError {}

/// Opaque evaluation context passed through the pipeline.
///
/// In an embedded context this can be a fixed-size struct on the stack.
/// In a `std` context it wraps a hash map for arbitrary key-value state.
pub struct EvalContext<'a> {
    /// Flat key-value pairs — avoids HashMap for `no_std` compatibility.
    pub slots: &'a [(&'static str, ContextValue)],
    /// Current logical time (for temporal engines). Nanoseconds since epoch,
    /// or a monotonic counter on embedded targets.
    pub logical_time: u64,
    /// Maximum recursion depth allowed for this evaluation.
    pub depth_limit: u8,
}

impl<'a> EvalContext<'a> {
    pub fn get(&self, key: &'static str) -> Option<&ContextValue> {
        self.slots.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
    }
}

/// Values that can appear in evaluation context slots.
#[derive(Debug, Clone, PartialEq)]
pub enum ContextValue {
    Bool(bool),
    Integer(i64),
    Float(f64),
    Str(&'static str),
    #[cfg(feature = "alloc")]
    OwnedStr(alloc::string::String),
}

impl ContextValue {
    pub fn as_bool(&self) -> Option<bool> {
        if let ContextValue::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        if let ContextValue::Integer(n) = self {
            Some(*n)
        } else {
            None
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        if let ContextValue::Float(f) = self {
            Some(*f)
        } else {
            None
        }
    }
}

/// The core trait every logic engine must implement.
///
/// Implementations are stateless by design — all state lives in `EvalContext`.
/// This makes engines safe to share across threads and suitable for ROM.
pub trait LogicEngine {
    /// Stable identifier for this engine instance.
    fn id(&self) -> EngineId;

    /// Which paradigm this engine primarily implements.
    fn paradigm(&self) -> Paradigm;

    /// Human-readable name for logic traces.
    fn name(&self) -> &'static str;

    /// Whether this engine can handle the given AST node.
    /// Called by the router before `evaluate` to avoid wasted work.
    fn can_handle(&self, node: &AstNode) -> bool;

    /// Evaluate an AST node and return a [`Verdict`].
    ///
    /// Engines must be **deterministic**: same `node` + same `ctx` → same `Verdict`.
    fn evaluate(&self, node: &AstNode, ctx: &EvalContext<'_>) -> Result<Verdict, EngineError>;
}
