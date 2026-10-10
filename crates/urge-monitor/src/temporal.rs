//! Continuous LTL formula monitoring.
//!
//! Each formula is a monitor that observes the world at successive instants and reports
//! whether the formula is satisfied, violated, or still undetermined. The predicates are
//! governance expressions; [`crate::GovernanceMonitor::observe`] evaluates them through
//! the pipeline against the facts of the instant. Until 0.1.4 nothing drove these
//! monitors: `GovernanceMonitor` held a list that was never fed.

/// An LTL formula being continuously monitored. Each predicate is a governance
/// expression, evaluated through the pipeline at every observation.
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

fn hstr<const N: usize>(s: &str) -> heapless::String<N> {
    let mut out = heapless::String::new();
    for c in s.chars() {
        if out.push(c).is_err() {
            break;
        }
    }
    out
}

impl LtlFormula {
    /// G(φ): `predicate` must hold at every observation. Truncated to 128 chars.
    pub fn globally(predicate: &str) -> Self {
        LtlFormula::Globally(hstr(predicate))
    }

    /// F(φ): `predicate` must hold at some observation, by `deadline_ns` if given.
    pub fn finally(predicate: &str, deadline_ns: Option<u64>) -> Self {
        LtlFormula::Finally {
            predicate: hstr(predicate),
            deadline_ns,
        }
    }

    /// φ U ψ: `phi` holds at every observation until one where `psi` holds, by
    /// `deadline_ns` if given. Each truncated to 64 chars.
    pub fn until(phi: &str, psi: &str, deadline_ns: Option<u64>) -> Self {
        LtlFormula::Until {
            phi: hstr(phi),
            psi: hstr(psi),
            deadline_ns,
        }
    }

    /// G(F(φ)): `predicate` must keep recurring. Never settles; see
    /// [`TemporalMonitor::last_satisfied_at`].
    pub fn globally_finally(predicate: &str) -> Self {
        LtlFormula::GloballyFinally(hstr(predicate))
    }

    /// The formula in LTL notation, for events and logs.
    pub fn notation(&self) -> heapless::String<160> {
        let mut s = heapless::String::new();
        let (a, b, c) = match self {
            LtlFormula::Globally(p) => ("G(", p.as_str(), ")"),
            LtlFormula::Finally { predicate, .. } => ("F(", predicate.as_str(), ")"),
            LtlFormula::Until { phi, .. } => ("(", phi.as_str(), ") U (…)"),
            LtlFormula::GloballyFinally(p) => ("G(F(", p.as_str(), "))"),
        };
        let _ = s.push_str(a);
        let _ = s.push_str(b);
        if let LtlFormula::Until { psi, .. } = self {
            let _ = s.push_str(") U (");
            let _ = s.push_str(psi.as_str());
            let _ = s.push(')');
        } else {
            let _ = s.push_str(c);
        }
        s
    }
}

/// Runtime state of an LTL monitor for one formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorState {
    Undetermined,
    Satisfied,
    Violated,
}

impl MonitorState {
    pub fn is_terminal(self) -> bool {
        self != MonitorState::Undetermined
    }
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

    /// Observe one instant. `holds` says whether a predicate (one of this formula's
    /// expressions) is true now; `now` is the logical time. Returns the new state. A
    /// terminal state is kept.
    ///
    /// G(φ) is never finished at a finite instant: it stays undetermined while φ holds
    /// and is violated the first time φ does not. (Until 0.1.4 it reported satisfied
    /// after one true observation.)
    pub fn observe(&mut self, holds: &dyn Fn(&str) -> bool, now: u64) -> MonitorState {
        if self.state.is_terminal() {
            return self.state;
        }

        match &self.formula {
            LtlFormula::Globally(p) => {
                if holds(p.as_str()) {
                    self.last_satisfied_at = Some(now);
                } else {
                    self.state = MonitorState::Violated;
                }
            }

            LtlFormula::Finally {
                predicate,
                deadline_ns,
            } => {
                if holds(predicate.as_str()) {
                    self.state = MonitorState::Satisfied;
                    self.last_satisfied_at = Some(now);
                } else if deadline_ns.is_some_and(|d| now > d) {
                    self.state = MonitorState::Violated;
                }
            }

            LtlFormula::Until {
                phi,
                psi,
                deadline_ns,
            } => {
                // Satisfied the instant ψ holds. While it does not, φ must hold at every
                // instant, and ψ must arrive by the deadline if there is one.
                if holds(psi.as_str()) {
                    self.state = MonitorState::Satisfied;
                    self.last_satisfied_at = Some(now);
                } else if !holds(phi.as_str()) || deadline_ns.is_some_and(|d| now > d) {
                    self.state = MonitorState::Violated;
                } else {
                    self.last_satisfied_at = Some(now);
                }
            }

            LtlFormula::GloballyFinally(p) => {
                // Liveness: never settles. `last_satisfied_at` tells a policy how long it
                // has been since φ last recurred.
                if holds(p.as_str()) {
                    self.last_satisfied_at = Some(now);
                }
            }
        }

        self.state
    }

    /// The one-predicate form of [`observe`](Self::observe): `predicate_holds` stands for
    /// the formula's predicate (ψ for `Until`, whose φ is then taken to hold).
    pub fn tick(&mut self, predicate_holds: bool, now: u64) -> MonitorState {
        let phi = match &self.formula {
            LtlFormula::Until { phi, .. } => Some(phi.clone()),
            _ => None,
        };
        self.observe(
            &|p| match &phi {
                Some(phi) if p == phi.as_str() => true,
                _ => predicate_holds,
            },
            now,
        )
    }
}
