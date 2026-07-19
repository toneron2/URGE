//! URGE side of the OPA comparison (`examples/comparison_opa/` at the repo
//! root holds the Rego side and the write-up).
//!
//! Scenario: a payment agent wants to run `submit_payment`. Policy:
//!   1. the agent must be authorized                      (deontic O)
//!   2. the compliance review must be complete throughout (temporal G)
//!   3. the review itself is an obligation with a deadline (lifecycle)
//!
//! OPA answers allow/deny for 1+2 collapsed into booleans. URGE evaluates
//! each constraint in its own logic, cross-checks the results, and — for the
//! deadline — tracks the obligation over time and emits a violation event.

use urge_core::engine::{ContextValue, EvalContext};
use urge_meta::{GovernancePipeline, PipelineConfig};
use urge_monitor::obligation::{Obligation, ObligationEvent, ObligationManager, ObligationType};

const POLICY: &str = "must authorized and always review_completed";

fn evaluate(pipeline: &GovernancePipeline, authorized: bool, review_completed: bool) {
    let slots = &[
        ("authorized", ContextValue::Bool(authorized)),
        ("review_completed", ContextValue::Bool(review_completed)),
    ];
    let ctx = EvalContext {
        slots,
        logical_time: 0,
        depth_limit: 16,
    };
    let v = pipeline.evaluate_str(POLICY, &ctx);

    println!(
        "input: authorized={authorized}, review_completed={review_completed}\n\
         → verdict:    {}\n\
         → formal:     {}\n\
         → confidence: {:.0}%\n\
         → consistent: {} (conflicts: {}{})",
        if v.valid { "PERMIT" } else { "DENY" },
        v.formal_notation,
        v.confidence.as_f32() * 100.0,
        v.cross_validation.consistent,
        v.cross_validation.conflicts_detected,
        v.cross_validation
            .conflict_detail
            .map(|d| format!(" — {d}"))
            .unwrap_or_default(),
    );
    println!("→ trace ({} entries), last stages:", v.trace.len());
    for e in v.trace.entries.iter().rev().take(4).rev() {
        println!("    [{:?}] {}", e.stage, e.description);
    }
    println!();
}

fn main() {
    println!("=== URGE side of the OPA comparison ===\n");
    println!("policy expression: {POLICY}\n");

    let pipeline = GovernancePipeline::new(PipelineConfig::healthcare());

    // Case 1: everything in order — same PERMIT OPA gives.
    evaluate(&pipeline, true, true);

    // Case 2: review missing. OPA says deny. URGE says deny AND reports the
    // cross-paradigm finding: the temporal constraint is violated while the
    // deontic obligation is active — a contradiction, not just a false.
    evaluate(&pipeline, true, false);

    // Part 2 — what OPA has no vocabulary for at all: the review is an
    // obligation with a deadline, tracked across time.
    println!("=== Obligation lifecycle (no OPA equivalent) ===\n");
    let mut mgr = ObligationManager::new();
    mgr.register(
        Obligation::new(
            "review-P123",
            ObligationType::Obligatory,
            "payment-agent",
            "complete_review",
            Some(1000), // deadline at t=1000
            0,
        ),
        0,
    );
    println!("t=0     obligation registered: complete_review, deadline t=1000");
    println!(
        "        active={} violated={}",
        mgr.active_count(),
        mgr.violated_count()
    );

    let violations = mgr.process(ObligationEvent::TimeTick { now: 2000 });
    println!("t=2000  time tick — deadline exceeded, review never happened");
    for v in &violations {
        println!("        VIOLATION EVENT: {v:?}");
    }
    println!(
        "        active={} violated={}",
        mgr.active_count(),
        mgr.violated_count()
    );
}
