//! The continuous governance monitor — ties obligations and temporal monitoring
//! to the instantaneous pipeline.

use crate::{
    obligation::{
        Obligation, ObligationEvent, ObligationId, ObligationManager, ObligationViolationEvent,
    },
    temporal::{LtlFormula, MonitorState, TemporalMonitor},
};
use urge_meta::GovernancePipeline;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
use urge_core::engine::{ContextValue, EvalContext};

/// An LTL monitor that became violated during [`GovernanceMonitor::observe`].
#[derive(Debug, Clone)]
pub struct LtlViolation {
    /// The index [`GovernanceMonitor::watch`] returned.
    pub monitor: usize,
    pub formula: LtlFormula,
    pub detected_at: u64,
}

/// The full continuous governance engine.
///
/// This is the system-level entry point for long-running governance over
/// event streams. Combine with your event bus or message queue.
pub struct GovernanceMonitor {
    pub pipeline: GovernancePipeline,
    pub obligations: ObligationManager,
    #[cfg(feature = "alloc")]
    pub temporal_monitors: Vec<TemporalMonitor>,
    #[cfg(not(feature = "alloc"))]
    pub temporal_monitors: heapless::Vec<TemporalMonitor, 32>,
    pub current_time: u64,
}

impl GovernanceMonitor {
    pub fn new(pipeline: GovernancePipeline) -> Self {
        GovernanceMonitor {
            pipeline,
            obligations: ObligationManager::new(),
            #[cfg(feature = "alloc")]
            temporal_monitors: Vec::new(),
            #[cfg(not(feature = "alloc"))]
            temporal_monitors: heapless::Vec::new(),
            current_time: 0,
        }
    }

    /// Advance logical time and process all deadline checks.
    #[cfg(feature = "alloc")]
    pub fn tick(&mut self, now: u64) -> Vec<ObligationViolationEvent> {
        self.current_time = now;
        self.obligations.process(ObligationEvent::TimeTick { now })
    }

    /// Register a new obligation to be tracked.
    pub fn track_obligation(&mut self, ob: Obligation) {
        let now = self.current_time;
        self.obligations.register(ob, now);
    }

    /// Start monitoring an LTL formula. Returns its index, reported in
    /// [`LtlViolation::monitor`]. (Bounded to 32 monitors without `alloc`; a 33rd is
    /// dropped and the index returned is past the end.)
    pub fn watch(&mut self, formula: LtlFormula) -> usize {
        let index = self.temporal_monitors.len();
        let monitor = TemporalMonitor::new(formula, self.current_time);
        #[cfg(feature = "alloc")]
        self.temporal_monitors.push(monitor);
        #[cfg(not(feature = "alloc"))]
        let _ = self.temporal_monitors.push(monitor);
        index
    }

    /// Observe the world at logical time `now`: every monitored formula's predicates
    /// are evaluated through the pipeline against `slots`, and the monitors that
    /// become violated at this instant are returned. Obligation deadlines are checked
    /// by [`tick`](Self::tick), not here.
    #[cfg(feature = "alloc")]
    pub fn observe(
        &mut self,
        now: u64,
        slots: &[(&'static str, ContextValue)],
    ) -> Vec<LtlViolation> {
        self.current_time = now;
        let ctx = EvalContext {
            slots,
            logical_time: now,
            depth_limit: self.pipeline.config.depth_limit,
        };
        let pipeline = &self.pipeline;
        let holds = |predicate: &str| pipeline.evaluate_str(predicate, &ctx).valid;
        let mut violations = Vec::new();
        for (index, monitor) in self.temporal_monitors.iter_mut().enumerate() {
            if monitor.state.is_terminal() {
                continue;
            }
            if monitor.observe(&holds, now) == MonitorState::Violated {
                violations.push(LtlViolation {
                    monitor: index,
                    formula: monitor.formula.clone(),
                    detected_at: now,
                });
            }
        }
        violations
    }

    /// Waive an obligation, recording who waived it. Whether `waived_by` may do so is
    /// the caller's governance decision (evaluate a policy first); the record is what
    /// this keeps. Returns whether an active obligation with that id was found.
    #[cfg(feature = "alloc")]
    pub fn waive(&mut self, obligation_id: &str, waived_by: &str) -> bool {
        let id = ObligationId::new(obligation_id);
        let found = self
            .obligations
            .obligations
            .iter()
            .any(|o| o.id == id && o.state.is_active());
        let mut by = heapless::String::new();
        for c in waived_by.chars().take(32) {
            let _ = by.push(c);
        }
        self.obligations.process(ObligationEvent::Waiver {
            obligation_id: id,
            waived_by: by,
            timestamp: self.current_time,
        });
        found
    }

    /// Report that `agent` attempted `action`: any active prohibition on it is
    /// violated and returned.
    #[cfg(feature = "alloc")]
    pub fn forbidden_attempt(
        &mut self,
        agent: &str,
        action: &str,
    ) -> Vec<ObligationViolationEvent> {
        let mut ag = heapless::String::new();
        let mut ac = heapless::String::new();
        for c in agent.chars().take(32) {
            let _ = ag.push(c);
        }
        for c in action.chars().take(64) {
            let _ = ac.push(c);
        }
        self.obligations.process(ObligationEvent::ForbiddenAttempt {
            agent: ag,
            action: ac,
            timestamp: self.current_time,
        })
    }

    /// Notify that an action was completed (may satisfy obligations).
    #[cfg(feature = "alloc")]
    pub fn action_completed(&mut self, agent: &str, action: &str) -> Vec<ObligationViolationEvent> {
        let mut ag = heapless::String::new();
        let mut ac = heapless::String::new();
        for c in agent.chars().take(32) {
            let _ = ag.push(c);
        }
        for c in action.chars().take(64) {
            let _ = ac.push(c);
        }
        self.obligations.process(ObligationEvent::ActionCompleted {
            agent: ag,
            action: ac,
            timestamp: self.current_time,
        })
    }

    /// Query current governance stats.
    pub fn stats(&self) -> MonitorStats {
        MonitorStats {
            active_obligations: self.obligations.active_count(),
            violated_obligations: self.obligations.violated_count(),
            active_ltl_monitors: self
                .temporal_monitors
                .iter()
                .filter(|m| !m.state.is_terminal())
                .count(),
            violated_ltl_monitors: self
                .temporal_monitors
                .iter()
                .filter(|m| m.state == MonitorState::Violated)
                .count(),
            current_time: self.current_time,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MonitorStats {
    pub active_obligations: usize,
    pub violated_obligations: usize,
    /// Monitors still undetermined. (Until 0.1.4 this counted every monitor.)
    pub active_ltl_monitors: usize,
    pub violated_ltl_monitors: usize,
    pub current_time: u64,
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
    use super::*;
    use crate::obligation::{ObligationState, ObligationType};
    use urge_meta::PipelineConfig;

    fn monitor() -> GovernanceMonitor {
        GovernanceMonitor::new(GovernancePipeline::new(PipelineConfig::default()))
    }

    #[test]
    fn a_globally_monitor_is_violated_once_and_stays_so() {
        let mut m = monitor();
        let g = m.watch(LtlFormula::globally("audit_active"));
        let up = &[("audit_active", ContextValue::Bool(true))];
        let down = &[("audit_active", ContextValue::Bool(false))];
        assert!(m.observe(1, up).is_empty());
        assert_eq!(m.stats().active_ltl_monitors, 1);
        let v = m.observe(2, down);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].monitor, g);
        assert_eq!(v[0].detected_at, 2);
        assert_eq!(v[0].formula.notation().as_str(), "G(audit_active)");
        assert!(m.observe(3, down).is_empty(), "reported once");
        assert_eq!(m.stats().violated_ltl_monitors, 1);
        assert_eq!(m.stats().active_ltl_monitors, 0);
    }

    #[test]
    fn a_finally_monitor_respects_its_deadline() {
        let mut m = monitor();
        m.watch(LtlFormula::finally("consent_obtained", Some(10)));
        let no = &[("consent_obtained", ContextValue::Bool(false))];
        let yes = &[("consent_obtained", ContextValue::Bool(true))];
        assert!(m.observe(5, no).is_empty());
        assert_eq!(m.observe(20, no).len(), 1, "deadline passed");
        let mut m = monitor();
        m.watch(LtlFormula::finally("consent_obtained", Some(10)));
        assert!(m.observe(5, yes).is_empty());
        assert_eq!(m.temporal_monitors[0].state, MonitorState::Satisfied);
        assert!(m.observe(20, no).is_empty(), "satisfied stays satisfied");
    }

    #[test]
    fn an_until_monitor_reads_both_sides() {
        let mut m = monitor();
        m.watch(LtlFormula::until("must escorted", "signed_out", None));
        let escorted = &[
            ("escorted", ContextValue::Bool(true)),
            ("signed_out", ContextValue::Bool(false)),
        ];
        let alone = &[
            ("escorted", ContextValue::Bool(false)),
            ("signed_out", ContextValue::Bool(false)),
        ];
        let out = &[
            ("escorted", ContextValue::Bool(false)),
            ("signed_out", ContextValue::Bool(true)),
        ];
        assert!(m.observe(1, escorted).is_empty());
        assert_eq!(m.temporal_monitors[0].state, MonitorState::Undetermined);
        assert_eq!(m.observe(2, alone).len(), 1, "φ failed before ψ");
        let mut m = monitor();
        m.watch(LtlFormula::until("must escorted", "signed_out", None));
        assert!(m.observe(1, escorted).is_empty());
        assert!(m.observe(2, out).is_empty());
        assert_eq!(m.temporal_monitors[0].state, MonitorState::Satisfied);
    }

    #[test]
    fn a_waiver_records_who_waived() {
        let mut m = monitor();
        m.track_obligation(Obligation::new(
            "ob-1",
            ObligationType::Obligatory,
            "nurse",
            "obtain_consent",
            Some(100),
            0,
        ));
        assert!(m.waive("ob-1", "supervisor_2"));
        let ob = &m.obligations.obligations[0];
        assert_eq!(ob.state, ObligationState::Waived);
        assert_eq!(ob.waived_by.as_deref(), Some("supervisor_2"));
        assert!(!m.waive("ob-1", "anyone"), "already terminal");
        assert!(!m.waive("missing", "anyone"));
    }

    #[test]
    fn a_forbidden_attempt_violates_the_prohibition() {
        let mut m = monitor();
        m.track_obligation(Obligation::new(
            "no-export",
            ObligationType::Forbidden,
            "intake-agent",
            "export_phi",
            None,
            0,
        ));
        assert!(m.forbidden_attempt("intake-agent", "read_phi").is_empty());
        let v = m.forbidden_attempt("intake-agent", "export_phi");
        assert_eq!(v.len(), 1);
        assert_eq!(m.stats().violated_obligations, 1);
    }
}
