//! Stage 4: Engine Router — the SWITCH statement of Figure 26.
//!
//! Given a `ParadigmSet`, the router selects the appropriate engine(s) and
//! dispatches AST nodes to them. This is **entirely deterministic**: the same
//! paradigm set always selects the same engines in the same order.
//!
//! There is no learned gating, no probabilistic selection, no neural routing.
//! This determinism is the key governance property: every routing decision is
//! fully auditable and reproducible.

use urge_core::{
    ast::AstNode,
    decision::{EntryOutcome, LogicTrace, Stage, TraceEntry, Verdict},
    engine::{EngineError, EvalContext, LogicEngine, Paradigm},
    symbol::ParadigmSet,
};
use urge_engines::all_engines;

#[cfg(feature = "alloc")]
use urge_core::{ast::Expr, decision::CrossValidation, symbol::SemanticClass};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// The one trace entry of a connective decided from its fragments. The validator passes such a
/// verdict through as it is: it already carries its sides' agreement and its parts' conflicts.
pub(crate) const DECOMPOSED: &str = "connective decided from its fragments";

/// One side's notation: its engines' notations, each once, joined with ∧.
#[cfg(feature = "alloc")]
fn side_notation(results: &[Result<Verdict, EngineError>]) -> alloc::string::String {
    let mut seen: Vec<&str> = Vec::new();
    for v in results.iter().filter_map(|v| v.as_ref().ok()) {
        let n = v.formal_notation.as_str();
        if !n.is_empty() && !seen.contains(&n) {
            seen.push(n);
        }
    }
    if seen.is_empty() {
        alloc::string::String::from("?")
    } else {
        seen.join(" ∧ ")
    }
}

/// What routing one node produced: its verdicts and, for a conjunction decided from its
/// fragments, the leaf engine verdicts under it. An enclosing `and` cross-validates those
/// leaves rather than one verdict per side, so `must a and always b and must c` is checked
/// over the same three leaves whichever way its conjuncts are ordered.
#[cfg(feature = "alloc")]
struct Routed {
    results: Vec<Result<Verdict, EngineError>>,
    conjuncts: Option<Vec<Verdict>>,
}

/// The engine router.
pub struct EngineRouter;

impl EngineRouter {
    /// Route an AST node to all capable engines and collect their verdicts.
    ///
    /// Routing priority:
    ///   Deontic > Temporal > Epistemic > Modal > Fuzzy > Paraconsistent > Boolean
    ///
    /// This ordering reflects governance priority: obligations take precedence
    /// over pure logical truth values.
    #[cfg(feature = "alloc")]
    pub fn route(
        node: &AstNode,
        active_paradigms: ParadigmSet,
        ctx: &EvalContext<'_>,
        trace: &mut LogicTrace,
    ) -> Vec<Result<Verdict, EngineError>> {
        Self::route_inner(node, active_paradigms, ctx, trace, 0).results
    }

    #[cfg(feature = "alloc")]
    fn route_inner(
        node: &AstNode,
        active_paradigms: ParadigmSet,
        ctx: &EvalContext<'_>,
        trace: &mut LogicTrace,
        depth: u8,
    ) -> Routed {
        let engines = all_engines();
        let mut results = Vec::new();

        for engine in &engines {
            // Only invoke engines whose paradigm is active.
            if !active_paradigms.contains(engine.paradigm()) {
                continue;
            }
            // Only invoke if engine claims it can handle this node.
            if !engine.can_handle(node) {
                continue;
            }

            trace.push(TraceEntry {
                stage: Stage::EngineRouting,
                paradigm: Some(engine.paradigm()),
                description: engine.name(),
                outcome: EntryOutcome::Routed,
            });

            let verdict = engine.evaluate(node, ctx);
            results.push(verdict);
        }

        // Mixed-paradigm decomposition (Figure 26, stage 5: "each selected
        // engine evaluates its AST fragment"). A boolean connective over
        // non-boolean children -- e.g. `must x and always y` -- is claimed only
        // by the Boolean engine, which cannot evaluate the deontic/temporal
        // children and errors out. In that case each side is routed as its own
        // fragment and decided by its own engines (and cross-validated within
        // itself), and the connective is decided from the two sides' results.
        //
        // ONE verdict comes back for the connective. Until 0.1.3 the sides'
        // verdicts were returned beside it as peers, so stage 6 voted over
        // different propositions and let a false deontic side deny the whole:
        // `must a or b` was decided as O(a) ∧ b, and a clean refusal reported
        // 60-80 % confidence. Now the verdict carries the connective's own
        // truth, the weaker side's engine agreement as its confidence, and, for
        // `and` only, the conflicts stage 6 finds between its parts: an `and`
        // asserts both sides at once, while `or`, `implies`, `iff` and `xor`
        // relate alternatives that are not checked against each other.
        if results.iter().all(|r| r.is_err()) && depth < ctx.depth_limit {
            if let Expr::Binary {
                op, left, right, ..
            } = node.as_ref()
            {
                // The temporal binaries decompose the same way: `must consent before
                // procedure` is O(consent) U procedure, whose sides the temporal engine
                // cannot evaluate itself. Each is decided at this instant as the temporal
                // engine decides boolean sides: U and W as ψ ∨ φ, R as ψ.
                let temporal = matches!(
                    op,
                    SemanticClass::Until | SemanticClass::Release | SemanticClass::WeakUntil
                );
                if temporal
                    || matches!(
                        op,
                        SemanticClass::Conjunction
                            | SemanticClass::Disjunction
                            | SemanticClass::Implication
                            | SemanticClass::Biconditional
                            | SemanticClass::ExclusiveOr
                    )
                {
                    let own_paradigm = if temporal {
                        Paradigm::Temporal
                    } else {
                        Paradigm::Boolean
                    };
                    trace.push(TraceEntry {
                        stage: Stage::EngineRouting,
                        paradigm: Some(own_paradigm),
                        description: "decomposing connective into paradigm fragments",
                        outcome: EntryOutcome::Routed,
                    });

                    let left = Self::route_inner(left, active_paradigms, ctx, trace, depth + 1);
                    let (left_valid, left_conf, _) =
                        crate::validator::CrossValidator::validate(&left.results, trace);
                    let right = Self::route_inner(right, active_paradigms, ctx, trace, depth + 1);
                    let (right_valid, right_conf, _) =
                        crate::validator::CrossValidator::validate(&right.results, trace);

                    let combined = match op {
                        SemanticClass::Conjunction => left_valid && right_valid,
                        SemanticClass::Disjunction => left_valid || right_valid,
                        SemanticClass::Implication => !left_valid || right_valid,
                        SemanticClass::Biconditional => left_valid == right_valid,
                        SemanticClass::ExclusiveOr => left_valid ^ right_valid,
                        SemanticClass::Until | SemanticClass::WeakUntil => {
                            right_valid || left_valid
                        }
                        SemanticClass::Release => right_valid,
                        _ => unreachable!(),
                    };
                    let (l, r) = (side_notation(&left.results), side_notation(&right.results));
                    let notation = match op {
                        SemanticClass::Conjunction => alloc::format!("{l} ∧ {r}"),
                        SemanticClass::Disjunction => alloc::format!("({l}) ∨ ({r})"),
                        SemanticClass::Implication => alloc::format!("({l}) → ({r})"),
                        SemanticClass::Biconditional => alloc::format!("({l}) ↔ ({r})"),
                        SemanticClass::ExclusiveOr => alloc::format!("({l}) ⊕ ({r})"),
                        SemanticClass::Until => alloc::format!("({l}) U ({r})"),
                        SemanticClass::WeakUntil => alloc::format!("({l}) W ({r})"),
                        _ => alloc::format!("({l}) R ({r})"),
                    };

                    // The parts an `and` cross-validates: a side that is itself a conjunction
                    // contributes its leaves, any other side its own verdicts.
                    let mut parts: Vec<Result<Verdict, EngineError>> = Vec::new();
                    for side in [left, right] {
                        match side.conjuncts {
                            Some(leaves) => parts.extend(leaves.into_iter().map(Ok)),
                            None => parts.extend(side.results),
                        }
                    }
                    let is_and = matches!(op, SemanticClass::Conjunction);
                    let cross = if is_and {
                        crate::validator::CrossValidator::validate(&parts, trace).2
                    } else {
                        CrossValidation::ok()
                    };
                    let mut paradigms = ParadigmSet::empty();
                    paradigms.insert(own_paradigm);
                    let mut citations = alloc::vec![];
                    for v in parts.iter().filter_map(|v| v.as_ref().ok()) {
                        for p in v.paradigms_evaluated.iter() {
                            paradigms.insert(p);
                        }
                        citations.extend(v.citations.iter().cloned());
                    }
                    let valid = combined && cross.consistent;
                    let mut own = LogicTrace::new();
                    own.push(TraceEntry {
                        stage: Stage::CrossValidation,
                        paradigm: Some(own_paradigm),
                        description: DECOMPOSED,
                        outcome: if valid {
                            EntryOutcome::Permitted
                        } else {
                            EntryOutcome::Denied
                        },
                    });
                    results.push(Ok(Verdict {
                        valid,
                        confidence: core::cmp::min(left_conf, right_conf),
                        paradigms_evaluated: paradigms,
                        trace: own,
                        cross_validation: cross,
                        formal_notation: notation,
                        citations,
                    }));
                    return Routed {
                        results,
                        conjuncts: is_and
                            .then(|| parts.into_iter().filter_map(Result::ok).collect()),
                    };
                }
            }
        }

        // If no specialized engine handled it, fall back to Boolean.
        if results.is_empty() {
            trace.push(TraceEntry {
                stage: Stage::EngineRouting,
                paradigm: Some(Paradigm::Boolean),
                description: "BooleanEngine (fallback)",
                outcome: EntryOutcome::Routed,
            });
            results.push(urge_engines::boolean::BooleanEngine.evaluate(node, ctx));
        }

        Routed {
            results,
            conjuncts: None,
        }
    }

    /// Embedded path: route to a single best-fit engine without allocation.
    /// Returns the verdict of the highest-priority matching engine.
    pub fn route_single(
        node: &AstNode,
        active_paradigms: ParadigmSet,
        ctx: &EvalContext<'_>,
        trace: &mut LogicTrace,
    ) -> Result<Verdict, EngineError> {
        let engines = all_engines();

        for engine in &engines {
            if active_paradigms.contains(engine.paradigm()) && engine.can_handle(node) {
                trace.push(TraceEntry {
                    stage: Stage::EngineRouting,
                    paradigm: Some(engine.paradigm()),
                    description: engine.name(),
                    outcome: EntryOutcome::Routed,
                });
                return engine.evaluate(node, ctx);
            }
        }

        // Boolean fallback.
        urge_engines::boolean::BooleanEngine.evaluate(node, ctx)
    }
}
