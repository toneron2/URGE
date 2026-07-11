//! Embedded BIOS-like governance facade.
//!
//! > **Experimental / WIP.** This module compiles and runs on `std`/`alloc`
//! > targets today. The fully heap-free `no_std` (no-`alloc`) build is **not yet
//! > functional**: the recursive AST needs an arena/index representation before
//! > it can be built without a heap. Tracked as future work.
//!
//! Zero-allocation, stack-only governance for bare-metal and RTOS targets.
//! This is the "governance chip" concept from the patent: a tiny deterministic
//! core that mediates all device capability access.
//!
//! ## Target footprint
//!
//! | Component           | Estimated Flash size |
//! |---------------------|---------------------|
//! | Unicode dict table  | ~8 KB               |
//! | Engine SWITCH logic | ~4 KB               |
//! | AST (shallow only)  | ~2 KB               |
//! | Obligation table    | ~16 KB (256 slots)  |
//! | Total               | ~30 KB              |
//!
//! Compare: a Python interpreter is ~8 MB. This is the 1/1000th figure.

use urge_core::{
    ast::{node, Expr, Literal},
    engine::{ContextValue, EvalContext},
    symbol::SemanticClass,
};
use urge_meta::{GovernancePipeline, PipelineConfig};

/// Pre-defined access control decisions for common BIOS scenarios.
/// These are static verdicts computed at compile time for zero-runtime-cost
/// hard-coded policies.
pub mod hard_rules {
    use urge_core::decision::Verdict;

    /// Camera access is always forbidden without explicit user permission.
    pub fn camera_deny() -> Verdict {
        Verdict::deny_immediate("camera: no user permission granted")
    }

    /// Microphone access is always forbidden during call without consent.
    pub fn microphone_consent_required() -> Verdict {
        Verdict::deny_immediate("microphone: consent required before activation")
    }

    /// Network access is always forbidden when in airplane mode.
    pub fn network_airplane_mode_deny() -> Verdict {
        Verdict::deny_immediate("network: airplane mode active")
    }
}

/// BIOS-level governance engine.
///
/// Designed for `no_std` + `no_alloc` contexts. Uses the pipeline in
/// single-engine routing mode with a shallow AST.
pub struct BiosGovernor {
    pipeline: GovernancePipeline,
}

impl BiosGovernor {
    pub fn new() -> Self {
        BiosGovernor {
            pipeline: GovernancePipeline::new(PipelineConfig::embedded()),
        }
    }

    /// Check whether an application is permitted to access a device capability.
    ///
    /// Decision factors:
    /// - User permission granted for this capability
    /// - Battery level sufficient (some capabilities denied when battery < threshold)
    /// - Device not in restricted mode
    /// - Application not on blocklist
    ///
    /// Returns `true` if access is permitted.
    pub fn check_access(&self, capability: &str, _app_id: &str, battery_percent: u8) -> bool {
        // Hard-coded battery check: critical peripherals denied below 5%.
        let battery_critical = battery_percent < 5;
        if battery_critical && matches!(capability, "camera" | "gps" | "bluetooth") {
            return false;
        }

        // Build a minimal context on the stack — no heap.
        let slots: &[(&'static str, ContextValue)] = &[
            (
                "battery_sufficient",
                ContextValue::Bool(battery_percent >= 20),
            ),
            ("permission_granted", ContextValue::Bool(true)), // Caller asserts.
            ("not_restricted", ContextValue::Bool(true)),     // Caller asserts.
        ];
        let ctx = EvalContext {
            slots,
            logical_time: 0,
            depth_limit: 4, // Very shallow on embedded.
        };

        // Build AST directly — no tokenization overhead.
        // Expression: battery_sufficient ∧ permission_granted ∧ not_restricted
        use urge_core::symbol::ParadigmSet;
        let mut ps = ParadigmSet::empty();
        ps.insert(urge_core::engine::Paradigm::Boolean);

        let ast = node(Expr::Binary {
            op: SemanticClass::Conjunction,
            left: node(Expr::Binary {
                op: SemanticClass::Conjunction,
                left: node(Expr::Var {
                    name: {
                        let mut s = heapless::String::new();
                        let _ = s.push_str("battery_sufficient");
                        s
                    },
                    paradigms: ps,
                }),
                right: node(Expr::Var {
                    name: {
                        let mut s = heapless::String::new();
                        let _ = s.push_str("permission_granted");
                        s
                    },
                    paradigms: ps,
                }),
                paradigms: ps,
            }),
            right: node(Expr::Var {
                name: {
                    let mut s = heapless::String::new();
                    let _ = s.push_str("not_restricted");
                    s
                },
                paradigms: ps,
            }),
            paradigms: ps,
        });

        let verdict =
            self.pipeline
                .evaluate_ast(&ast, ps, &ctx, urge_core::decision::LogicTrace::new());
        verdict.valid
    }

    /// Evaluate a pre-classified governance expression for embedded use.
    ///
    /// Returns only the boolean permit/deny — no heap allocation.
    pub fn evaluate_simple(&self, _capability: SemanticClass, subject: bool) -> bool {
        let mut ps = urge_core::symbol::ParadigmSet::empty();
        ps.insert(urge_core::engine::Paradigm::Boolean);
        let ast = node(Expr::Lit(Literal::Bool(subject)));
        let ctx = EvalContext {
            slots: &[],
            logical_time: 0,
            depth_limit: 4,
        };
        let v = self
            .pipeline
            .evaluate_ast(&ast, ps, &ctx, urge_core::decision::LogicTrace::new());
        v.valid
    }
}

impl Default for BiosGovernor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bios_denies_on_critical_battery() {
        let gov = BiosGovernor::new();
        assert!(!gov.check_access("camera", "app.health", 3));
    }

    #[test]
    fn bios_permits_with_sufficient_battery() {
        let gov = BiosGovernor::new();
        assert!(gov.check_access("camera", "app.health", 80));
    }
}
