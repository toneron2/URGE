//! # urge-monitor — Continuous Governance
//!
//! While `urge-meta` handles *instantaneous* governance decisions,
//! `urge-monitor` handles *continuous* governance over time.
//!
//! ## The key innovation over traditional policy engines
//!
//! Traditional engines: Request → Evaluate → Allow/Deny (stateless)
//! URGE monitor:        Event → Track Obligations → Enforce Over Time (stateful)
//!
//! ## Healthcare example
//!
//! ```text
//! T+0s:   patient_admission(P123) fires
//!         → CREATE Obligation(obtain_consent, P123, deadline=T+86400s)
//!         → CREATE Obligation(audit_all_access, P123, forever)
//!         → State: { P123: { consent: PENDING, audit: ACTIVE } }
//!
//! T+3600s: access_phi(P123, nurse_007) fires
//!          → CHECK G(audit_all_access, P123) → PASS (audit running)
//!          → LOG access event
//!
//! T+86401s: time_tick fires (deadline exceeded)
//!           → CHECK O(obtain_consent, P123) deadline
//!           → VIOLATION: transition PENDING → VIOLATED
//!           → EMIT ViolationEvent { id, agent, action, reason }
//!           → ESCALATE to supervisor
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod engine;
pub mod obligation;
pub mod temporal;

pub use engine::GovernanceMonitor;
pub use obligation::{Obligation, ObligationId, ObligationState, ObligationType};
pub use temporal::{LtlFormula, TemporalMonitor};
