//! # urge-runtime
//!
//! The `std`-tier public API for URGE. This is the crate most applications
//! will depend on directly. It assembles all lower crates into a coherent,
//! easy-to-use governance interface.
//!
//! ## Quick start — healthcare
//!
//! ```rust,no_run
//! use urge_runtime::healthcare::HealthcareGovernor;
//!
//! let mut gov = HealthcareGovernor::new();
//!
//! // Register HIPAA obligation: must obtain consent within 24 hours.
//! gov.require_consent("P123", "nurse_007", 86_400_000_000_000); // 24h in ns
//!
//! // Tick time forward — check for violations.
//! let now_ns: u64 = 1_000_000_000; // supply your own clock source
//! let violations = gov.tick(now_ns);
//! for v in &violations {
//!     eprintln!("Violation: {:?}", v);
//! }
//!
//! // Evaluate a governance expression before an action.
//! // `evaluate` takes the expression plus a slice of context slots.
//! let verdict = gov.evaluate("must audit_access", &[]);
//! assert!(verdict.valid, "Access denied by governance layer");
//! ```
//!
//! ## Quick start — embedded BIOS access control
//!
//! ```rust
//! use urge_runtime::embedded::BiosGovernor;
//!
//! let gov = BiosGovernor::new();
//! // All evaluation is no-alloc, stack-only.
//! let permitted = gov.check_access("camera", "app.health", 45); // 45% battery
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod audit;
pub mod embedded;
pub mod healthcare;

// Re-export the full crate surface so users need only one dep.
pub use urge_core::{AstNode, Expr, Literal, Paradigm, Verdict};
pub use urge_meta::{GovernancePipeline, PipelineConfig};
pub use urge_monitor::{GovernanceMonitor, Obligation, ObligationState, ObligationType};
