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

use urge_core::{
    decision::{Citation, Verdict},
    engine::{ContextValue, EvalContext},
};
use urge_meta::{GovernancePipeline, PipelineConfig};
use urge_monitor::{
    obligation::{Obligation, ObligationType, ObligationViolationEvent},
    GovernanceMonitor,
};

use crate::audit::AuditLog;

/// A governance rule anchored to the source that requires it.
///
/// [`HealthcareGovernor::evaluate_policy`] attaches the source to the verdict as a
/// [`Citation`] and to the audit entry, so a decision can be traced to the provision
/// that produced it. Until 0.1.4 the rules were bare expression strings and no verdict
/// ever carried a citation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    /// The governance expression.
    pub expression: &'static str,
    /// Short citation id, e.g. `HIPAA-§164.312(a)(1)`.
    pub citation: &'static str,
    /// What the cited provision requires, in one sentence.
    pub requirement: &'static str,
}

impl Policy {
    /// The citation this policy anchors a verdict to.
    pub fn cite(&self) -> Citation {
        Citation {
            id: self.citation.into(),
            description: self.requirement.into(),
        }
    }
}

/// Pre-built HIPAA compliance rules, each anchored to its section of 45 CFR 164.
pub mod hipaa {
    use super::Policy;

    /// Minimum Necessary: access limited to the minimum necessary for the purpose.
    pub const MIN_NECESSARY: Policy = Policy {
        expression: "must minimum_necessary_access",
        citation: "HIPAA-§164.502(b)",
        requirement: "Uses and disclosures of PHI are limited to the minimum necessary to accomplish the intended purpose.",
    };

    /// Access Control: technical policies that allow access only to authorized users.
    pub const ACCESS_CONTROL: Policy = Policy {
        expression: "must authorized_user and must authenticated",
        citation: "HIPAA-§164.312(a)(1)",
        requirement: "Technical policies and procedures allow access to ePHI only to persons granted access rights.",
    };

    /// Audit Controls: mechanisms that record and examine activity on ePHI systems.
    pub const AUDIT_CONTROLS: Policy = Policy {
        expression: "always audit_active",
        citation: "HIPAA-§164.312(b)",
        requirement: "Hardware, software and procedural mechanisms record and examine activity in systems that contain ePHI.",
    };

    /// Integrity: PHI must not be improperly altered or destroyed.
    pub const DATA_INTEGRITY: Policy = Policy {
        expression: "always phi_integrity_maintained",
        citation: "HIPAA-§164.312(c)",
        requirement: "ePHI is protected from improper alteration or destruction.",
    };

    /// Transmission Security: guard against unauthorized access during transmission.
    pub const TRANSMISSION_SECURITY: Policy = Policy {
        expression: "must encrypted_transmission",
        citation: "HIPAA-§164.312(e)(1)",
        requirement:
            "ePHI transmitted over an electronic network is guarded against unauthorized access.",
    };

    /// PHI access as [`HealthcareGovernor::check_phi_access`](super::HealthcareGovernor::check_phi_access)
    /// decides it: access control and audit controls together.
    pub const PHI_ACCESS: Policy = Policy {
        expression: "must authorized_user and must authenticated and always audit_active",
        citation: "HIPAA-§164.312(a)(1); HIPAA-§164.312(b)",
        requirement: "Access to ePHI is granted only to authorized, authenticated users while audit controls record the access.",
    };
}

/// Pre-built clinical protocol rules.
pub mod clinical {
    use super::Policy;

    /// Informed consent must be obtained before any clinical procedure: a procedure
    /// obliges consent. (Until 0.1.4 this read `must consent_obtained before procedure`,
    /// which no engine could evaluate, so it denied in every context.)
    pub const INFORMED_CONSENT: Policy = Policy {
        expression: "procedure implies must consent_obtained",
        citation: "clinical:informed-consent",
        requirement: "A clinical procedure is performed only with the patient's informed consent.",
    };

    /// PHQ-9 score >= 15 triggers escalation. (Kroenke et al. 2001)
    pub const PHQ9_SEVERE_ESCALATION: Policy = Policy {
        expression: "must escalate_to_provider",
        citation: "PHQ-9:Kroenke-2001",
        requirement:
            "A PHQ-9 score of 15 or more (moderately severe depression) is escalated to a provider.",
    };

    /// Medication administration requires order verification.
    pub const MED_ADMIN_ORDER: Policy = Policy {
        expression: "must verified_order and must authenticated",
        citation: "clinical:medication-order-verification",
        requirement: "Medication is administered only against a verified order by an authenticated clinician.",
    };
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
    ) -> Verdict {
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

    /// Evaluate a [`Policy`]: its expression against `context_slots`, with the policy's
    /// citation on the verdict and on the audit entry, which also records the acting
    /// `agent` and a `correlation_id` (a patient or request id) when given.
    pub fn evaluate_policy(
        &mut self,
        policy: &Policy,
        context_slots: &[(&'static str, ContextValue)],
        agent: Option<&str>,
        correlation_id: Option<&str>,
    ) -> Verdict {
        let ctx = EvalContext {
            slots: context_slots,
            logical_time: self.current_time_ns,
            depth_limit: 16,
        };
        let mut verdict = self.monitor.pipeline.evaluate_str(policy.expression, &ctx);
        verdict.citations.push(policy.cite());
        self.audit.record_with(
            policy.expression,
            &verdict,
            self.current_time_ns,
            correlation_id,
            agent,
        );
        verdict
    }

    /// Check HIPAA access control before allowing a provider to access PHI: the
    /// [`hipaa::PHI_ACCESS`] policy, audited under `patient_id` with `agent_id` as the
    /// actor. Until 0.1.4 the agent, the patient and `audit_active` were ignored.
    ///
    /// Returns `Ok(())` if permitted, `Err(denial_reason)` if denied.
    pub fn check_phi_access(
        &mut self,
        agent_id: &str,
        patient_id: &str,
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
        let verdict =
            self.evaluate_policy(&hipaa::PHI_ACCESS, slots, Some(agent_id), Some(patient_id));
        if verdict.valid {
            Ok(())
        } else if !is_authorized || !is_authenticated {
            Err("HIPAA §164.312(a)(1): access denied, user not authorized or not authenticated")
        } else {
            Err("HIPAA §164.312(b): access denied, audit controls are not active")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn informed_consent_gates_a_procedure() {
        let mut gov = HealthcareGovernor::new();
        let case = |gov: &mut HealthcareGovernor, procedure: bool, consent: bool| {
            gov.evaluate(
                clinical::INFORMED_CONSENT.expression,
                &[
                    ("procedure", ContextValue::Bool(procedure)),
                    ("consent_obtained", ContextValue::Bool(consent)),
                ],
            )
            .valid
        };
        assert!(!case(&mut gov, true, false), "a procedure without consent");
        assert!(case(&mut gov, true, true));
        assert!(case(&mut gov, false, false), "no procedure, nothing owed");
    }
}
