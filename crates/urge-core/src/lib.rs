//! # urge-core
//!
//! The `no_std`-compatible kernel of the Universal Reasoning Governance Engine.
//!
//! ## Architecture
//!
//! Based on the provisional patent "Unicode Semantic Dictionary and Multi-Logic
//! Processing Architecture for Modular Reasoning Systems" (TODOMODO.IO AGENCY LLC,
//! Anthony R. Slosar). The core concept, captured in **Figure 26** of the patent,
//! is a deterministic pipeline:
//!
//! ```text
//! Input
//!   │
//!   ▼
//! [Tokenizer]  ─── Unicode Semantic Dictionary ───► Operator symbols mapped to paradigms
//!   │
//!   ▼
//! [Paradigm Detector]  ──────────────────────────► Set of active LogicParadigms
//!   │
//!   ▼
//! [AST Builder]  ────────────────────────────────► Typed expression tree
//!   │
//!   ▼
//! [Engine Router / SWITCH]  ─────────────────────► Per-paradigm engine selected
//!   │
//!   ▼
//! [Cross-System Validator]  ─────────────────────► Contradiction detection
//!   │
//!   ▼
//! Verdict { valid, confidence, trace, formal_notation }
//! ```
//!
//! Each stage is fully auditable. Every routing and validation decision is captured
//! in the `LogicTrace`, making the entire reasoning chain inspectable after the fact.
//!
//! ## Deployment targets
//!
//! | Feature flags      | Target                                   |
//! |--------------------|------------------------------------------|
//! | (none / heapless)  | Bare-metal firmware, BIOS-like chip      |
//! | `alloc`            | RTOS, microkernel with allocator         |
//! | `std`              | Linux/macOS/Windows process              |
//!
//! The `alloc` feature enables heap-allocated AST nodes. Without it, expression
//! depth is bounded by the fixed-size `heapless` structures.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod ast;
pub mod decision;
pub mod engine;
pub mod symbol;

// Re-export the key public surface
pub use ast::{AstNode, Expr, Literal};
pub use decision::{Confidence, CrossValidation, LogicTrace, TraceEntry, Verdict};
pub use engine::{EngineError, EngineId, LogicEngine, Paradigm};
pub use symbol::{SemanticClass, Symbol, UnicodeSemanticDictionary};
