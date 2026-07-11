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
