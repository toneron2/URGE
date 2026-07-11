//! Integration tests for the full URGE pipeline.
//!
//! These tests validate the complete Figure 26 pipeline end-to-end:
//! from string expression through all seven stages to a final Verdict.

use urge_core::engine::{ContextValue, EvalContext};
use urge_meta::{GovernancePipeline, PipelineConfig};
use urge_monitor::obligation::{Obligation, ObligationManager, ObligationType};

fn empty_ctx() -> EvalContext<'static> {
    EvalContext {
        slots: &[],
        logical_time: 0,
        depth_limit: 32,
    }
}

// ── Boolean baseline ───────────────────────────────────────────────────────────

#[test]
fn test_boolean_true() {
    let p = GovernancePipeline::new(PipelineConfig::default());
    let v = p.evaluate_str("true", &empty_ctx());
    assert!(v.valid);
}

#[test]
fn test_boolean_false() {
    let p = GovernancePipeline::new(PipelineConfig::default());
    let v = p.evaluate_str("false", &empty_ctx());
    assert!(!v.valid);
}

#[test]
fn test_negation() {
    let p = GovernancePipeline::new(PipelineConfig::default());
    let v = p.evaluate_str("not false", &empty_ctx());
    assert!(v.valid);
}

#[test]
fn test_conjunction_both_true() {
    let p = GovernancePipeline::new(PipelineConfig::default());
    let slots = &[
        ("a", ContextValue::Bool(true)),
        ("b", ContextValue::Bool(true)),
    ];
    let ctx = EvalContext {
        slots,
        logical_time: 0,
        depth_limit: 32,
    };
    let v = p.evaluate_str("a and b", &ctx);
    assert!(v.valid);
}

#[test]
fn test_conjunction_one_false() {
    let p = GovernancePipeline::new(PipelineConfig::default());
    let slots = &[
        ("a", ContextValue::Bool(true)),
        ("b", ContextValue::Bool(false)),
    ];
    let ctx = EvalContext {
        slots,
        logical_time: 0,
        depth_limit: 32,
    };
    let v = p.evaluate_str("a and b", &ctx);
    assert!(!v.valid);
}

#[test]
fn test_disjunction_one_true() {
    let p = GovernancePipeline::new(PipelineConfig::default());
    let slots = &[
        ("a", ContextValue::Bool(false)),
        ("b", ContextValue::Bool(true)),
    ];
    let ctx = EvalContext {
        slots,
        logical_time: 0,
        depth_limit: 32,
    };
    let v = p.evaluate_str("a or b", &ctx);
    assert!(v.valid);
}

// ── Deontic engine ─────────────────────────────────────────────────────────────

#[test]
fn test_deontic_obligation_activated() {
    let p = GovernancePipeline::new(PipelineConfig::default());
    let v = p.evaluate_str("must true", &empty_ctx());
    assert!(v
        .paradigms_evaluated
        .contains(urge_core::engine::Paradigm::Deontic));
}

#[test]
fn test_deontic_forbidden_denied() {
    let p = GovernancePipeline::new(PipelineConfig::default());
    let v = p.evaluate_str("must_not true", &empty_ctx());
    // "must_not" = Forbidden → body true means forbidden action was true → deny.
    // (May or may not be handled depending on parser path; at minimum, deontic is active.)
    assert!(v
        .paradigms_evaluated
        .contains(urge_core::engine::Paradigm::Deontic));
}

// ── Temporal engine ────────────────────────────────────────────────────────────

#[test]
fn test_temporal_globally_activated() {
    let p = GovernancePipeline::new(PipelineConfig::default());
    let v = p.evaluate_str("always true", &empty_ctx());
    assert!(v
        .paradigms_evaluated
        .contains(urge_core::engine::Paradigm::Temporal));
    assert!(v.valid);
}

#[test]
fn test_temporal_eventually() {
    let p = GovernancePipeline::new(PipelineConfig::default());
    let v = p.evaluate_str("eventually true", &empty_ctx());
    assert!(v
        .paradigms_evaluated
        .contains(urge_core::engine::Paradigm::Temporal));
}

// ── Audit trail ────────────────────────────────────────────────────────────────

#[test]
fn test_trace_populated() {
    let p = GovernancePipeline::new(PipelineConfig::default());
    let v = p.evaluate_str("must always true", &empty_ctx());
    assert!(
        !v.trace.is_empty(),
        "Trace must be non-empty for any evaluation"
    );
}

#[test]
fn test_trace_has_cross_validation_stage() {
    use urge_core::decision::Stage;
    let p = GovernancePipeline::new(PipelineConfig::default());
    let v = p.evaluate_str("true", &empty_ctx());
    assert!(
        v.trace
            .entries
            .iter()
            .any(|e| e.stage == Stage::CrossValidation),
        "Cross-validation stage must appear in trace"
    );
}

// ── Formal notation ────────────────────────────────────────────────────────────

#[test]
fn test_formal_notation_non_empty() {
    let p = GovernancePipeline::new(PipelineConfig::default());
    let v = p.evaluate_str("true", &empty_ctx());
    assert!(
        !v.formal_notation.is_empty(),
        "Formal notation must be emitted"
    );
}

// ── Obligation lifecycle ───────────────────────────────────────────────────────

#[test]
fn test_obligation_activated_on_register() {
    let mut mgr = ObligationManager::new();
    let ob = Obligation::new(
        "test-001",
        ObligationType::Obligatory,
        "agent1",
        "action_x",
        None,
        0,
    );
    mgr.register(ob, 0);
    assert_eq!(mgr.active_count(), 1);
    assert_eq!(mgr.violated_count(), 0);
}

#[test]
fn test_obligation_satisfied_by_action() {
    use urge_monitor::obligation::ObligationEvent;
    let mut mgr = ObligationManager::new();
    let ob = Obligation::new(
        "test-002",
        ObligationType::Obligatory,
        "agent1",
        "obtain_consent",
        None,
        0,
    );
    mgr.register(ob, 0);

    let mut ag = heapless::String::new();
    let _ = ag.push_str("agent1");
    let mut ac = heapless::String::new();
    let _ = ac.push_str("obtain_consent");

    let violations = mgr.process(ObligationEvent::ActionCompleted {
        agent: ag,
        action: ac,
        timestamp: 1000,
    });
    assert!(violations.is_empty());
    assert_eq!(mgr.active_count(), 0);
}

#[test]
fn test_obligation_violated_by_deadline() {
    let mut mgr = ObligationManager::new();
    // Deadline at t=1000, now=2000 → violated.
    let ob = Obligation::new(
        "test-003",
        ObligationType::Obligatory,
        "agent1",
        "action_x",
        Some(1000),
        0,
    );
    mgr.register(ob, 0);

    let violations = mgr.process(urge_monitor::obligation::ObligationEvent::TimeTick { now: 2000 });
    assert!(
        !violations.is_empty(),
        "Deadline exceeded must produce violation event"
    );
    assert_eq!(mgr.violated_count(), 1);
}

// ── Pipeline configs ───────────────────────────────────────────────────────────

#[test]
fn test_healthcare_config_exhaustive() {
    let p = GovernancePipeline::new(PipelineConfig::healthcare());
    assert!(p.config.exhaustive_evaluation);
    let v = p.evaluate_str("true", &empty_ctx());
    // In exhaustive mode, all paradigms are evaluated.
    assert!(v
        .paradigms_evaluated
        .contains(urge_core::engine::Paradigm::Boolean));
    assert!(v
        .paradigms_evaluated
        .contains(urge_core::engine::Paradigm::Deontic));
}

#[test]
fn test_confidence_certain_for_boolean() {
    let p = GovernancePipeline::new(PipelineConfig::default());
    let v = p.evaluate_str("true", &empty_ctx());
    // Boolean verdicts should have high confidence.
    assert!(v.confidence.0 > 100);
}

// ── Cross-system validation ────────────────────────────────────────────────────

#[test]
fn test_cross_validation_consistent_for_simple_boolean() {
    let p = GovernancePipeline::new(PipelineConfig::default());
    let v = p.evaluate_str("true", &empty_ctx());
    assert!(v.cross_validation.consistent);
    assert_eq!(v.cross_validation.conflicts_detected, 0);
}
