//! Agentic governance gate.
//!
//! Demonstrates URGE as a governance layer for a multi-agent system.
//! Before any agent action is executed, the action is submitted to the
//! governance pipeline. The pipeline evaluates deontic, temporal, and
//! epistemic constraints to determine if the action is permitted.
//!
//! This is the "BROAD" use case: autonomous agents in a healthcare ERP
//! operating under deterministic formal logic governance.

use urge_core::engine::{ContextValue, EvalContext};
use urge_meta::{GovernancePipeline, PipelineConfig};

/// Simulated agent action request.
struct AgentAction {
    agent_id: &'static str,
    action: &'static str,
    is_authorized: bool,
    audit_running: bool,
    /// The governance expression that must evaluate to `true` for the action to proceed.
    governance_expr: &'static str,
}

fn main() {
    println!("=== URGE Agent Governance Gate Demo ===\n");

    let pipeline = GovernancePipeline::new(PipelineConfig::healthcare());

    let actions = &[
        AgentAction {
            agent_id: "billing-agent",
            action: "submit_claim",
            is_authorized: true,
            audit_running: true,
            governance_expr: "must authorized and always audit_running",
        },
        AgentAction {
            agent_id: "rx-agent",
            action: "prescribe_medication",
            is_authorized: false, // Not authorized — should be denied.
            audit_running: true,
            governance_expr: "must authorized and must verified_order",
        },
        AgentAction {
            agent_id: "intake-agent",
            action: "access_phi",
            is_authorized: true,
            audit_running: false, // Audit not running — HIPAA violation.
            governance_expr: "must authorized and always audit_running",
        },
        AgentAction {
            agent_id: "scheduler-agent",
            action: "book_appointment",
            is_authorized: true,
            audit_running: true,
            governance_expr: "must authorized",
        },
    ];

    println!("{:<20} {:<25} {:<12}", "Agent", "Action", "Decision");
    println!("{}", "-".repeat(60));

    for action in actions {
        let slots: &[(&'static str, ContextValue)] = &[
            ("authorized", ContextValue::Bool(action.is_authorized)),
            ("audit_running", ContextValue::Bool(action.audit_running)),
            ("verified_order", ContextValue::Bool(false)), // Default: not verified.
        ];

        let ctx = EvalContext {
            slots,
            logical_time: 0,
            depth_limit: 16,
        };

        let verdict = pipeline.evaluate_str(action.governance_expr, &ctx);

        println!(
            "{:<20} {:<25} {}",
            action.agent_id,
            action.action,
            if verdict.valid {
                "PERMITTED ✓"
            } else {
                "DENIED    ✗"
            },
        );

        if !verdict.valid {
            println!(
                "  Confidence: {:.0}% | Conflicts: {} | Paradigms: {}",
                verdict.confidence.as_f32() * 100.0,
                verdict.cross_validation.conflicts_detected,
                verdict.paradigms_evaluated.iter().count(),
            );
            println!("  Formal: {}", verdict.formal_notation);
            println!("  Trace ({} steps):", verdict.trace.len());
            for entry in verdict.trace.entries.iter().take(3) {
                println!("    [{:?}] {:?}", entry.stage, entry.description);
            }
        }
    }

    println!("\n=== Agent governance summary ===");
    println!("  Every agent action is gated by formal logic, not behavioral alignment.");
    println!("  The governance layer is: deterministic, auditable, sub-millisecond.");
    println!("  No LLM inference involved in permit/deny decisions.");
    println!("  This is the URGE architecture operating as designed.");
}
