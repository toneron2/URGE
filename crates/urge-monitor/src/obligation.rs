//! Obligation lifecycle state machine.
//!
//! This is the data layer for continuous deontic monitoring. Every
//! `DeonticStatement` with a `deadline_ns` creates an `Obligation` entry
//! in the monitor's world state.
//!
//! ## State machine
//!
//! ```text
//!          ┌─────────────────────────────────────────────────────┐
//!          │                                                     │
//!     ─► PENDING ──► ACTIVE ──► SATISFIED                       │
//!                      │                                         │
//!                      ├──► VIOLATED ──► (escalation actions)    │
//!                      │                                         │
//!                      ├──► WAIVED                               │
//!                      │                                         │
//!                      └──► EXPIRED                              │
//!                                                                │
//!  Transitions are irreversible once VIOLATED, WAIVED, EXPIRED   │
//!  SATISFIED is terminal (obligation discharged)                  │
//!          └─────────────────────────────────────────────────────┘
//! ```

use heapless::String as HString;

/// Stable identifier for an obligation (up to 36 chars — UUID-sized).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObligationId(pub HString<36>);

impl ObligationId {
    pub fn new(s: &str) -> Self {
        let mut inner = HString::new();
        for c in s.chars().take(36) {
            let _ = inner.push(c);
        }
        ObligationId(inner)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// The SDL modality of this obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ObligationType {
    Obligatory, // O(φ): must happen
    Permitted,  // P(φ): may happen
    Forbidden,  // F(φ): must not happen
}

/// Lifecycle state of a single obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ObligationState {
    /// Created but not yet active (waiting for trigger condition).
    Pending,
    /// Currently active — being tracked against deadline.
    Active,
    /// The obligatory action was performed. Terminal.
    Satisfied,
    /// Deadline exceeded without satisfaction, or prohibited action occurred. Terminal.
    Violated,
    /// Obligation formally waived by an authorized agent. Terminal.
    Waived,
    /// Deadline reached, obligation no longer enforceable (different from Violated
    /// in jurisdictions that distinguish lapse from breach). Terminal.
    Expired,
}

impl ObligationState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            ObligationState::Satisfied
                | ObligationState::Violated
                | ObligationState::Waived
                | ObligationState::Expired
        )
    }

    pub fn is_active(self) -> bool {
        matches!(self, ObligationState::Pending | ObligationState::Active)
    }
}

/// A tracked obligation in the world state.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Obligation {
    pub id: ObligationId,

    /// SDL modality.
    pub obligation_type: ObligationType,

    /// The agent bearing the obligation.
    pub agent: HString<32>,

    /// The action the obligation concerns.
    pub action: HString<64>,

    /// Optional object/patient of the action.
    pub object: Option<HString<64>>,

    /// Current lifecycle state.
    pub state: ObligationState,

    /// Logical time when obligation was created (nanoseconds or counter).
    pub created_at: u64,

    /// Deadline for satisfaction. `None` = no deadline (permanent).
    pub deadline_ns: Option<u64>,

    /// Logical time of last state transition.
    pub last_updated: u64,

    /// Policy source citation.
    pub source: Option<HString<128>>,

    /// Priority (higher = more urgent). Used for escalation ordering.
    pub priority: u8,
}

impl Obligation {
    pub fn new(
        id: &str,
        obligation_type: ObligationType,
        agent: &str,
        action: &str,
        deadline_ns: Option<u64>,
        created_at: u64,
    ) -> Self {
        let mut agent_s = HString::new();
        for c in agent.chars().take(32) {
            let _ = agent_s.push(c);
        }
        let mut action_s = HString::new();
        for c in action.chars().take(64) {
            let _ = action_s.push(c);
        }

        Obligation {
            id: ObligationId::new(id),
            obligation_type,
            agent: agent_s,
            action: action_s,
            object: None,
            state: ObligationState::Pending,
            created_at,
            deadline_ns,
            last_updated: created_at,
            source: None,
            priority: 128,
        }
    }

    /// Transition to Active.
    pub fn activate(&mut self, now: u64) {
        if self.state == ObligationState::Pending {
            self.state = ObligationState::Active;
            self.last_updated = now;
        }
    }

    /// Mark as satisfied (action performed).
    pub fn satisfy(&mut self, now: u64) {
        if self.state.is_active() {
            self.state = ObligationState::Satisfied;
            self.last_updated = now;
        }
    }

    /// Check deadline and transition to Violated if exceeded.
    /// Returns true if a violation was newly detected.
    pub fn check_deadline(&mut self, now: u64) -> bool {
        if self.state.is_terminal() {
            return false;
        }
        if let Some(deadline) = self.deadline_ns {
            if now > deadline {
                self.state = match self.obligation_type {
                    ObligationType::Obligatory => ObligationState::Violated,
                    ObligationType::Permitted => ObligationState::Expired,
                    ObligationType::Forbidden => ObligationState::Expired,
                };
                self.last_updated = now;
                return true;
            }
        }
        false
    }

    /// Waive this obligation.
    pub fn waive(&mut self, now: u64) {
        if self.state.is_active() {
            self.state = ObligationState::Waived;
            self.last_updated = now;
        }
    }

    /// Human-readable summary for audit logs.
    pub fn summary(&self) -> HString<128> {
        let mut s = HString::new();
        let _ = s.push_str(self.id.as_str());
        let _ = s.push(':');
        let state_str = match self.state {
            ObligationState::Pending => "PENDING",
            ObligationState::Active => "ACTIVE",
            ObligationState::Satisfied => "SATISFIED",
            ObligationState::Violated => "VIOLATED",
            ObligationState::Waived => "WAIVED",
            ObligationState::Expired => "EXPIRED",
        };
        let _ = s.push_str(state_str);
        s
    }
}

/// Events that drive obligation lifecycle transitions.
#[derive(Debug, Clone)]
pub enum ObligationEvent {
    /// An action was completed — may satisfy matching obligations.
    ActionCompleted {
        agent: HString<32>,
        action: HString<64>,
        timestamp: u64,
    },
    /// A time tick — drives deadline checking.
    TimeTick { now: u64 },
    /// An explicit waiver from an authorized agent.
    Waiver {
        obligation_id: ObligationId,
        waived_by: HString<32>,
        timestamp: u64,
    },
    /// A forbidden action was attempted.
    ForbiddenAttempt {
        agent: HString<32>,
        action: HString<64>,
        timestamp: u64,
    },
}

/// Events emitted by the obligation manager for external consumers.
#[derive(Debug, Clone)]
pub enum ObligationViolationEvent {
    DeadlineExceeded {
        id: ObligationId,
        agent: HString<32>,
        action: HString<64>,
        deadline_ns: u64,
        detected_at: u64,
    },
    ForbiddenActionAttempted {
        id: ObligationId,
        agent: HString<32>,
        action: HString<64>,
        timestamp: u64,
    },
}

/// The obligation manager — holds and tracks all active obligations.
///
/// Bounded to 256 obligations in embedded mode.
#[cfg(feature = "alloc")]
pub struct ObligationManager {
    pub obligations: alloc::vec::Vec<Obligation>,
}

#[cfg(not(feature = "alloc"))]
pub struct ObligationManager {
    pub obligations: heapless::Vec<Obligation, 256>,
}

impl ObligationManager {
    pub fn new() -> Self {
        ObligationManager {
            #[cfg(feature = "alloc")]
            obligations: alloc::vec::Vec::new(),
            #[cfg(not(feature = "alloc"))]
            obligations: heapless::Vec::new(),
        }
    }

    pub fn register(&mut self, mut ob: Obligation, now: u64) {
        ob.activate(now);
        #[cfg(feature = "alloc")]
        self.obligations.push(ob);
        #[cfg(not(feature = "alloc"))]
        let _ = self.obligations.push(ob);
    }

    /// Process an incoming event and return any violation events that result.
    #[cfg(feature = "alloc")]
    pub fn process(&mut self, event: ObligationEvent) -> alloc::vec::Vec<ObligationViolationEvent> {
        let mut violations = alloc::vec::Vec::new();

        match &event {
            ObligationEvent::TimeTick { now } => {
                for ob in self.obligations.iter_mut() {
                    if ob.check_deadline(*now) {
                        if let Some(deadline) = ob.deadline_ns {
                            violations.push(ObligationViolationEvent::DeadlineExceeded {
                                id: ob.id.clone(),
                                agent: ob.agent.clone(),
                                action: ob.action.clone(),
                                deadline_ns: deadline,
                                detected_at: *now,
                            });
                        }
                    }
                }
            }

            ObligationEvent::ActionCompleted {
                agent,
                action,
                timestamp,
            } => {
                for ob in self.obligations.iter_mut() {
                    if ob.state.is_active() && ob.agent == *agent && ob.action == *action {
                        ob.satisfy(*timestamp);
                    }
                }
            }

            ObligationEvent::Waiver {
                obligation_id,
                timestamp,
                ..
            } => {
                for ob in self.obligations.iter_mut() {
                    if &ob.id == obligation_id {
                        ob.waive(*timestamp);
                        break;
                    }
                }
            }

            ObligationEvent::ForbiddenAttempt {
                agent,
                action,
                timestamp,
            } => {
                for ob in self.obligations.iter_mut() {
                    if ob.state.is_active()
                        && ob.obligation_type == ObligationType::Forbidden
                        && ob.agent == *agent
                        && ob.action == *action
                    {
                        ob.state = ObligationState::Violated;
                        ob.last_updated = *timestamp;
                        violations.push(ObligationViolationEvent::ForbiddenActionAttempted {
                            id: ob.id.clone(),
                            agent: ob.agent.clone(),
                            action: ob.action.clone(),
                            timestamp: *timestamp,
                        });
                    }
                }
            }
        }

        violations
    }

    pub fn active_count(&self) -> usize {
        self.obligations
            .iter()
            .filter(|o| o.state.is_active())
            .count()
    }

    pub fn violated_count(&self) -> usize {
        self.obligations
            .iter()
            .filter(|o| o.state == ObligationState::Violated)
            .count()
    }
}

impl Default for ObligationManager {
    fn default() -> Self {
        Self::new()
    }
}
