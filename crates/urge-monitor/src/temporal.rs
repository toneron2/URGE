//! Continuous LTL formula monitoring.
//!
//! Implements runtime LTL monitoring using the classic automaton-based approach.
//! Each formula is compiled to a monitor that accepts event streams and
//! determines (at each step) whether the formula is satisfied, violated, or
//! still undetermined.

/// An LTL formula being continuously monitored.
#[derive(Debug, Clone)]
pub enum LtlFormula {
    /// G(φ): φ must hold at every future point.
    Globally(heapless::String<128>),
    /// F(φ, deadline): φ must hold before deadline.
    Finally {
        predicate: heapless::String<128>,
        deadline_ns: Option<u64>,
    },
    /// φ U ψ: φ holds until ψ becomes true.
    Until {
        phi: heapless::String<64>,
        psi: heapless::String<64>,
        deadline_ns: Option<u64>,
    },
    /// G(F(φ)): φ must happen infinitely often. Used for liveness properties.
    GloballyFinally(heapless::String<64>),
}

/// Runtime state of an LTL monitor for one formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorState {
    Undetermined,
    Satisfied,
    Violated,
}

/// An active LTL monitor tracking one formula.
pub struct TemporalMonitor {
    pub formula: LtlFormula,
    pub state: MonitorState,
    pub created_at: u64,
    pub last_satisfied_at: Option<u64>,
}

impl TemporalMonitor {
    pub fn new(formula: LtlFormula, now: u64) -> Self {
        TemporalMonitor {
            formula,
            state: MonitorState::Undetermined,
            created_at: now,
            last_satisfied_at: None,
        }
    }

    /// Feed an event to the monitor. Returns new state.
    ///
    /// `predicate_holds`: whether the formula's inner predicate is currently true.
    /// `now`: current logical time.
    pub fn tick(&mut self, predicate_holds: bool, now: u64) -> MonitorState {
        if self.state == MonitorState::Violated {
            return MonitorState::Violated;
        }

        match &self.formula {
            LtlFormula::Globally(_) => {
                // G(φ): violated immediately if φ is ever false.
                if !predicate_holds {
                    self.state = MonitorState::Violated;
                } else {
                    self.state = MonitorState::Satisfied;
                    self.last_satisfied_at = Some(now);
                }
            }

            LtlFormula::Finally { deadline_ns, .. } => {
                if predicate_holds {
                    self.state = MonitorState::Satisfied;
                    self.last_satisfied_at = Some(now);
                } else if let Some(d) = deadline_ns {
                    if now > *d {
                        self.state = MonitorState::Violated;
                    }
                    // else: still Undetermined — deadline not reached.
                }
                // No deadline: always Undetermined until satisfied.
            }

            LtlFormula::Until {
                psi: _,
                deadline_ns,
                phi: _,
            } => {
                // φ U ψ:
                // - Satisfied when ψ becomes true.
                // - Violated if ψ never becomes true and deadline exceeded.
                // (We use predicate_holds to stand in for ψ in this simplified version.)
                if predicate_holds {
                    self.state = MonitorState::Satisfied;
                } else if let Some(d) = deadline_ns {
                    if now > *d {
                        self.state = MonitorState::Violated;
                    }
                }
            }

            LtlFormula::GloballyFinally(_) => {
                // G(F(φ)): φ must happen again eventually.
                // Reset satisfaction timer each time φ holds.
                if predicate_holds {
                    self.last_satisfied_at = Some(now);
                }
                // If too much time since last satisfaction, could be violated.
                // (Policy-defined "too much time" not encoded here — use deadline on Finally.)
                self.state = MonitorState::Undetermined; // Ongoing liveness.
            }
        }

        self.state
    }
}
