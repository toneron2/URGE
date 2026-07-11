//! Logic engine implementations for all supported paradigms.
//!
//! Each module implements [`urge_core::LogicEngine`] for one paradigm.
//! All engines are stateless and `Send + Sync` by design.
//!
//! ## Engine registry
//!
//! The [`registry`] function returns a slice of all built-in engines.
//! The meta-engine (in `urge-meta`) uses this to populate its router.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod boolean;
pub mod deontic;
pub mod epistemic;
pub mod fuzzy;
pub mod modal;
pub mod paraconsistent;
pub mod temporal;

use urge_core::LogicEngine;

/// Returns references to all built-in engines.
///
/// The order here determines priority in the cross-validation stage:
/// higher-precedence engines appear first. Deontic before Boolean reflects the
/// governance-first design: obligations override raw boolean truth.
pub fn all_engines() -> [&'static dyn LogicEngine; 7] {
    [
        &deontic::DeonticEngine,
        &temporal::TemporalEngine,
        &epistemic::EpistemicEngine,
        &modal::ModalEngine,
        &fuzzy::FuzzyEngine,
        &paraconsistent::ParaconsistentEngine,
        &boolean::BooleanEngine,
    ]
}
