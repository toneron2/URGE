//! Healthcare-specific governance facade.
//!
//! Provides HIPAA and clinical protocol compliance as first-class primitives,
//! implementing the common patterns from BROAD (Behavioral Reasoning Over
//! Agentic Domains) healthcare ERP.
//!
//! ## Design philosophy
//!
//! Rules are NOT arbitrary developer opinions. Every governance rule in this
//! module is anchored to a regulatory or clinical source:
//! - HIPAA §164.312 (access control)
//! - HIPAA §164.528 (accounting of disclosures)
//! - APA Practice Guidelines
//! - Kroenke et al. PHQ-9 validation (clinical thresholds)
//!
//! The goal: when an auditor asks "why did the system allow/deny X?",
//! the answer traces to a citable, authoritative source.

use urge_core::engine::{ContextValue, EvalContext};
use urge_meta::{GovernancePipeline, PipelineConfig};
use urge_monitor::{
    obligation::{Obligation, ObligationType, ObligationViolationEvent},
    GovernanceMonitor,
};

use crate::audit::AuditLog;

/// Pre-built HIPAA compliance rules as governance expressions.
pub mod hipaa {
    /// HIPAA Minimum Necessary: access must be limited to the minimum necessary
    /// to accomplish the intended purpose. §164.502(b)
    pub const MIN_NECESSARY: &str = "must minimum_necessary_access";

    /// HIPAA Access Control: covered entities must implement technical policies
    /// that allow access only to authorized users. §164.312(a)(1)
    pub const ACCESS_CONTROL: &str = "must authorized_user and must authenticated";

    /// HIPAA Audit Controls: hardware, software, and procedural mechanisms to
    /// record and examine activity. §164.312(b)
    pub const AUDIT_CONTROLS: &str = "always audit_active";

    /// HIPAA Integrity: PHI must not be improperly altered or destroyed. §164.312(c)
    pub const DATA_INTEGRITY: &str = "always phi_integrity_maintained";

    /// HIPAA Transmission Security: guard against unauthorized access during
    /// transmission. §164.312(e)(1)
    pub const TRANSMISSION_SECURITY: &str = "must encrypted_transmission";
}

/// Pre-built clinical protocol rules.
pub mod clinical {
    /// Informed consent must be obtained before any clinical procedure.
    pub const INFORMED_CONSENT: &str = "must consent_obtained before procedure";

    /// PHQ-9 score >= 15 triggers escalation. (Kroenke et al. 2001)
    pub const PHQ9_SEVERE_ESCALATION: &str = "must escalate_to_provider";

    /// Medication administration requires order verification.
    pub const MED_ADMIN_ORDER: &str = "must verified_order and must authenticated";
}

/// The healthcare governance system — HIPAA + clinical + audit, combined.
pub struct HealthcareGovernor {
    monitor: GovernanceMonitor,
    audit: AuditLog,
    current_time_ns: u64,
}

impl HealthcareGovernor {
    pub fn new() -> Self {
        let pipeline = GovernancePipeline::new(PipelineConfig::healthcare());
        let monitor = GovernanceMonitor::new(pipeline);
        HealthcareGovernor {
            monitor,
            audit: AuditLog::new(),
            current_time_ns: 0,
        }
    }

    /// Register a consent obligation for a patient. Must be satisfied within `deadline_ns`.
    pub fn require_consent(&mut self, patient_id: &str, responsible_agent: &str, deadline_ns: u64) {
        let id = {
            #[cfg(feature = "alloc")]
            {
                alloc::format!("consent:{}:{}", patient_id, responsible_agent)
            }
            #[cfg(not(feature = "alloc"))]
            {
                "consent:obligation"
            }
        };

        let ob = Obligation::new(
            &id,
            ObligationType::Obligatory,
            responsible_agent,
            "obtain_consent",
            Some(self.current_time_ns + deadline_ns),
            self.current_time_ns,
        );
        self.monitor.track_obligation(ob);
    }

    /// Evaluate whether an action is permitted under current governance context.
    ///
    /// Builds context from the provided key-value pairs and runs the
    /// full Figure 26 pipeline.
    pub fn evaluate(
        &mut self,
        expression: &str,
        context_slots: &[(&'static str, ContextValue)],
    ) -> urge_core::decision::Verdict {
        let ctx = EvalContext {
            slots: context_slots,
            logical_time: self.current_time_ns,
            depth_limit: 16,
        };
        let verdict = self.monitor.pipeline.evaluate_str(expression, &ctx);
        self.audit
            .record(expression, &verdict, self.current_time_ns, None);
        verdict
    }

    /// Check HIPAA access control before allowing a provider to access PHI.
    ///
    /// Returns `Ok(())` if permitted, `Err(denial_reason)` if denied.
    pub fn check_phi_access(
        &mut self,
        _agent_id: &str,
        _patient_id: &str,
        is_authorized: bool,
        is_authenticated: bool,
        audit_active: bool,
    ) -> Result<(), &'static str> {
        let slots: &[(&'static str, ContextValue)] = &[
            ("authorized_user", ContextValue::Bool(is_authorized)),
            ("authenticated", ContextValue::Bool(is_authenticated)),
            ("audit_active", ContextValue::Bool(audit_active)),
            ("minimum_necessary_access", ContextValue::Bool(true)), // Caller asserts this.
        ];
        let verdict = self.evaluate(hipaa::ACCESS_CONTROL, slots);
        if verdict.valid {
            Ok(())
        } else {
            Err("HIPAA access control denied: insufficient authorization or authentication")
        }
    }

    /// Advance time and check for obligation deadline violations.
    pub fn tick(&mut self, now_ns: u64) -> alloc::vec::Vec<ObligationViolationEvent> {
        self.current_time_ns = now_ns;
        self.monitor.tick(now_ns)
    }

    /// Record that a clinical action was completed (satisfies matching obligations).
    pub fn action_completed(
        &mut self,
        agent: &str,
        action: &str,
    ) -> alloc::vec::Vec<ObligationViolationEvent> {
        self.monitor.action_completed(agent, action)
    }

    pub fn audit_log(&self) -> &AuditLog {
        &self.audit
    }

    pub fn stats(&self) -> urge_monitor::engine::MonitorStats {
        self.monitor.stats()
    }
}

impl Default for HealthcareGovernor {
    fn default() -> Self {
        Self::new()
    }
}
