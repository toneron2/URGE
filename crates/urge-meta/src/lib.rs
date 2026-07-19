//! # urge-meta — The Figure 26 Pipeline
//!
//! This crate is the operational heart of URGE: it implements the complete
//! Adaptive Processing Workflow — the Figure 26 pipeline, named for the
//! diagram in the original design document.
//!
//! ## Pipeline stages (direct mapping to Figure 26)
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────┐
//! │                    ADAPTIVE PROCESSING WORKFLOW                       │
//! │                         (Figure 26)                                  │
//! ├──────────────────────────────────────────────────────────────────────┤
//! │  STAGE 1: TOKENIZATION                                               │
//! │    Input text → Token stream via Unicode Semantic Dictionary         │
//! │    Every symbol classified by paradigm before any evaluation         │
//! ├──────────────────────────────────────────────────────────────────────┤
//! │  STAGE 2: PARADIGM DETECTION                                         │
//! │    Token paradigm annotations → ParadigmSet (compact bitset)         │
//! │    Determines which engines will be activated                        │
//! ├──────────────────────────────────────────────────────────────────────┤
//! │  STAGE 3: AST CONSTRUCTION                                           │
//! │    Tokens → typed expression tree (Pratt parser)                     │
//! │    Each node annotated with its paradigm(s)                          │
//! ├──────────────────────────────────────────────────────────────────────┤
//! │  STAGE 4: ENGINE ROUTING (SWITCH)                                    │
//! │    For each active paradigm → route AST to matching engine           │
//! │    Deterministic dispatch — no learned gating, no inference          │
//! ├──────────────────────────────────────────────────────────────────────┤
//! │  STAGE 5: ENGINE EVALUATION                                          │
//! │    Each selected engine evaluates its portion of the AST             │
//! │    Returns partial Verdict with local confidence and trace           │
//! ├──────────────────────────────────────────────────────────────────────┤
//! │  STAGE 6: CROSS-SYSTEM VALIDATION               ◄── DIFFERENTIATOR  │
//! │    All engine verdicts checked for inter-paradigm consistency        │
//! │    Contradictions detected, flagged, and resolved                    │
//! ├──────────────────────────────────────────────────────────────────────┤
//! │  STAGE 7: VERDICT SYNTHESIS                                          │
//! │    Aggregate confidence, merge traces, produce final Verdict         │
//! │    Formal notation emitted for audit                                 │
//! └──────────────────────────────────────────────────────────────────────┘
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod parser;
pub mod pipeline;
pub mod router;
pub mod tokenizer;
pub mod validator;

pub use pipeline::{GovernancePipeline, PipelineConfig};
pub use tokenizer::Tokenizer;
