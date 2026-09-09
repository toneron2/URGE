//! # URGE — Universal Reasoning Governance Engine
//!
//! An LLM agent can act. It usually cannot prove why the action was permitted.
//! URGE closes that gap: wrap an agent action in a formal policy check and get
//! back a verdict with the reasoning that produced it. The same input yields the
//! same verdict every time, and the derivation is attached rather than inferred.
//!
//! This crate is a **facade**. It re-exports [`urge_runtime`], which assembles
//! the lower crates into one interface. Depend on `urge` unless you need a
//! narrower slice:
//!
//! | Crate | What it holds |
//! |---|---|
//! | [`urge-core`](https://docs.rs/urge-core) | Types, AST, verdicts. No framework dependencies. |
//! | [`urge-engines`](https://docs.rs/urge-engines) | The per-paradigm evaluators. |
//! | [`urge-meta`](https://docs.rs/urge-meta) | Paradigm detection and the governance pipeline. |
//! | [`urge-monitor`](https://docs.rs/urge-monitor) | Deontic obligation lifecycle over time. |
//! | [`urge-runtime`](https://docs.rs/urge-runtime) | The `std`-tier API this crate re-exports. |
//!
//! ## Paradigms
//!
//! Seven are implemented and cross-validated against each other: boolean, modal
//! (S5), epistemic, deontic, temporal (LTL), fuzzy and paraconsistent (Belnap
//! four-valued). Probabilistic is on the roadmap and is **not** implemented.
//!
//! ## No-std
//!
//! Default features enable `std`. For embedded targets disable them and enable
//! `alloc`, which is the floor for this crate:
//!
//! ```toml
//! urge = { version = "0.1", default-features = false, features = ["alloc"] }
//! ```
//!
//! `alloc` is required, not optional: [`urge_runtime`] does not build without
//! it. If you need the allocation-free tier, depend on
//! [`urge-core`](https://docs.rs/urge-core) directly rather than on this
//! facade.
//!
//! ## Try it without installing anything
//!
//! There is a live browser demo at
//! <https://toneron2.github.io/URGE/demo/> — type a governance expression, flip
//! context slots, and watch the verdict, the formal notation and the reasoning
//! trace update.
//!
//! Licensed under Apache-2.0. Source at <https://github.com/toneron2/URGE>.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

pub use urge_runtime::*;
