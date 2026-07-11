//! The continuous governance monitor — ties obligations and temporal monitoring
//! to the instantaneous pipeline.

use crate::{
    obligation::{Obligation, ObligationEvent, ObligationManager, ObligationViolationEvent},
    temporal::TemporalMonitor,
};
use urge_meta::GovernancePipeline;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

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
            active_ltl_monitors: self.temporal_monitors.len(),
            current_time: self.current_time,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MonitorStats {
    pub active_obligations: usize,
    pub violated_obligations: usize,
    pub active_ltl_monitors: usize,
    pub current_time: u64,
}
