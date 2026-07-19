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
use urge_core::{
    ast::Expr,
    decision::{Confidence, CrossValidation},
    symbol::SemanticClass,
};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

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
        Self::route_inner(node, active_paradigms, ctx, trace, 0)
    }

    #[cfg(feature = "alloc")]
    fn route_inner(
        node: &AstNode,
        active_paradigms: ParadigmSet,
        ctx: &EvalContext<'_>,
        trace: &mut LogicTrace,
        depth: u8,
    ) -> Vec<Result<Verdict, EngineError>> {
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
        // non-boolean children — e.g. `must x and always y` — is claimed only
        // by the Boolean engine, which cannot evaluate the deontic/temporal
        // children and errors out. In that case, route each side as its own
        // fragment so every paradigm's engine sees its fragment, then append a
        // boolean-skeleton verdict combining the sides so the connective's
        // truth-functional semantics are preserved. The fragment verdicts stay
        // in the result set, which is what lets stage 6 cross-check paradigms
        // against each other.
        if results.iter().all(|r| r.is_err()) && depth < ctx.depth_limit {
            if let Expr::Binary {
                op, left, right, ..
            } = node.as_ref()
            {
                if matches!(
                    op,
                    SemanticClass::Conjunction
                        | SemanticClass::Disjunction
                        | SemanticClass::Implication
                        | SemanticClass::Biconditional
                        | SemanticClass::ExclusiveOr
                ) {
                    trace.push(TraceEntry {
                        stage: Stage::EngineRouting,
                        paradigm: Some(Paradigm::Boolean),
                        description: "decomposing connective into paradigm fragments",
                        outcome: EntryOutcome::Routed,
                    });

                    let left_results =
                        Self::route_inner(left, active_paradigms, ctx, trace, depth + 1);
                    let (left_valid, _, _) =
                        crate::validator::CrossValidator::validate(&left_results, trace);
                    let right_results =
                        Self::route_inner(right, active_paradigms, ctx, trace, depth + 1);
                    let (right_valid, _, _) =
                        crate::validator::CrossValidator::validate(&right_results, trace);

                    let combined = match op {
                        SemanticClass::Conjunction => left_valid && right_valid,
                        SemanticClass::Disjunction => left_valid || right_valid,
                        SemanticClass::Implication => !left_valid || right_valid,
                        SemanticClass::Biconditional => left_valid == right_valid,
                        SemanticClass::ExclusiveOr => left_valid ^ right_valid,
                        _ => unreachable!(),
                    };

                    let mut skeleton_paradigms = ParadigmSet::empty();
                    skeleton_paradigms.insert(Paradigm::Boolean);

                    results.extend(left_results);
                    results.extend(right_results);
                    // Empty notation: the fragment verdicts already carry the
                    // per-paradigm notations, which synthesis joins with ∧.
                    results.push(Ok(Verdict {
                        valid: combined,
                        confidence: Confidence::CERTAIN,
                        paradigms_evaluated: skeleton_paradigms,
                        trace: LogicTrace::new(),
                        cross_validation: CrossValidation::ok(),
                        formal_notation: alloc::string::String::new(),
                        citations: alloc::vec![],
                    }));
                    return results;
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

        results
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
