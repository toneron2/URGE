//! The complete Figure 26 pipeline as a single callable unit.
//!
//! `GovernancePipeline::evaluate()` drives all seven stages in sequence and
//! returns a final `Verdict`. This is the public entry point for all governance
//! decisions — whether on a BIOS chip or a healthcare ERP agentic layer.
//!
//! ## Timing
//!
//! Target: <50ms for typical governance decisions (10-50 active rules).
//! Actual observed performance in the shell proof: ~1ms for simple rule chains.
//! Rust implementation: expected <100µs on x86, <5ms on ARM Cortex-M4.

use urge_core::{
    ast::AstNode,
    decision::{EntryOutcome, LogicTrace, Stage, TraceEntry, Verdict},
    engine::{EvalContext, Paradigm},
    symbol::{ParadigmSet, UnicodeSemanticDictionary},
};

use crate::{
    parser::Parser, router::EngineRouter, tokenizer::Tokenizer, validator::CrossValidator,
};

#[cfg(feature = "alloc")]
use alloc::string::String;

/// Configuration for the governance pipeline.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Maximum AST recursion depth. Limits resource consumption on embedded.
    pub depth_limit: u8,
    /// If true, always run all engines regardless of paradigm detection.
    /// Useful for audit/compliance scenarios where every paradigm must certify.
    pub exhaustive_evaluation: bool,
    /// Minimum confidence threshold. Verdicts below this threshold are denied.
    pub confidence_threshold: u8,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        PipelineConfig {
            depth_limit: 32,
            exhaustive_evaluation: false,
            confidence_threshold: 128, // ~0.50
        }
    }
}

impl PipelineConfig {
    /// Conservative config for healthcare/HIPAA contexts:
    /// exhaustive evaluation, higher confidence threshold.
    pub fn healthcare() -> Self {
        PipelineConfig {
            depth_limit: 16,
            exhaustive_evaluation: true,
            confidence_threshold: 204, // ~0.80
        }
    }

    /// Minimal config for embedded BIOS-like contexts:
    /// fast, shallow evaluation.
    pub fn embedded() -> Self {
        PipelineConfig {
            depth_limit: 8,
            exhaustive_evaluation: false,
            confidence_threshold: 51, // ~0.20
        }
    }
}

/// The complete governance pipeline (Figure 26).
pub struct GovernancePipeline {
    pub config: PipelineConfig,
    tokenizer: Tokenizer,
}

impl GovernancePipeline {
    pub fn new(config: PipelineConfig) -> Self {
        GovernancePipeline {
            config,
            tokenizer: Tokenizer::new(),
        }
    }

    pub fn default_healthcare() -> Self {
        Self::new(PipelineConfig::healthcare())
    }

    pub fn default_embedded() -> Self {
        Self::new(PipelineConfig::embedded())
    }

    /// Evaluate a governance expression given as a string.
    ///
    /// Full pipeline: tokenize → detect → parse → route → evaluate → validate → synthesize.
    ///
    /// This is the primary API for string-based governance expressions.
    #[cfg(feature = "alloc")]
    pub fn evaluate_str(&self, expression: &str, ctx: &EvalContext<'_>) -> Verdict {
        let mut trace = LogicTrace::new();

        // ── Stage 1: Tokenization ──────────────────────────────────────────
        trace.push(TraceEntry {
            stage: Stage::Tokenization,
            paradigm: None,
            description: "tokenizing input",
            outcome: EntryOutcome::Evaluated,
        });
        let tokens = self.tokenizer.tokenize(expression);

        if tokens.is_empty() {
            return Verdict::deny_immediate("empty or unrecognized expression");
        }

        // ── Stage 2: Paradigm Detection ────────────────────────────────────
        trace.push(TraceEntry {
            stage: Stage::ParadigmDetection,
            paradigm: None,
            description: "detecting active paradigms",
            outcome: EntryOutcome::Evaluated,
        });
        let classes: alloc::vec::Vec<_> = tokens.iter().map(|t| t.class).collect();
        let active_paradigms = if self.config.exhaustive_evaluation {
            // All paradigms active in exhaustive mode.
            let mut all = ParadigmSet::empty();
            for &p in Paradigm::ALL {
                all.insert(p);
            }
            all
        } else {
            UnicodeSemanticDictionary::detect_paradigms(&classes)
        };

        for p in active_paradigms.iter() {
            trace.push(TraceEntry {
                stage: Stage::ParadigmDetection,
                paradigm: Some(p),
                description: p.name(),
                outcome: EntryOutcome::Evaluated,
            });
        }

        // ── Stage 3: AST Construction ──────────────────────────────────────
        trace.push(TraceEntry {
            stage: Stage::AstConstruction,
            paradigm: None,
            description: "building AST",
            outcome: EntryOutcome::Evaluated,
        });
        let mut parser = Parser::new(tokens);
        let ast = match parser.parse() {
            Some(a) => a,
            None => return Verdict::deny_immediate("failed to parse expression"),
        };

        self.evaluate_ast(&ast, active_paradigms, ctx, trace)
    }

    /// Evaluate a pre-built AST directly.
    ///
    /// Use this when you construct governance expressions programmatically
    /// (e.g., from a rule database) to skip tokenization and parsing overhead.
    pub fn evaluate_ast(
        &self,
        ast: &AstNode,
        active_paradigms: ParadigmSet,
        ctx: &EvalContext<'_>,
        mut trace: LogicTrace,
    ) -> Verdict {
        // ── Stage 4 + 5: Engine Routing and Evaluation ────────────────────
        trace.push(TraceEntry {
            stage: Stage::EngineRouting,
            paradigm: None,
            description: "routing to engines",
            outcome: EntryOutcome::Evaluated,
        });

        #[cfg(feature = "alloc")]
        let verdicts = EngineRouter::route(ast, active_paradigms, ctx, &mut trace);

        #[cfg(not(feature = "alloc"))]
        let single_verdict = EngineRouter::route_single(ast, active_paradigms, ctx, &mut trace);

        // ── Stage 6: Cross-System Validation ──────────────────────────────
        trace.push(TraceEntry {
            stage: Stage::CrossValidation,
            paradigm: None,
            description: "cross-system validation",
            outcome: EntryOutcome::Evaluated,
        });

        #[cfg(feature = "alloc")]
        let (final_valid, confidence, cross_validation) =
            CrossValidator::validate(&verdicts, &mut trace);

        #[cfg(not(feature = "alloc"))]
        let (final_valid, confidence, cross_validation) = match &single_verdict {
            Ok(v) => {
                let cv = CrossValidator::validate_single(v, &mut trace);
                (v.valid, v.confidence, cv)
            }
            Err(_) => (
                false,
                Confidence::NONE,
                CrossValidation {
                    consistent: false,
                    conflicts_detected: 1,
                    conflict_detail: Some("engine error"),
                },
            ),
        };

        // Apply confidence threshold.
        let threshold_met = confidence.0 >= self.config.confidence_threshold;

        // ── Stage 7: Verdict Synthesis ─────────────────────────────────────
        trace.push(TraceEntry {
            stage: Stage::VerdictSynthesis,
            paradigm: None,
            description: if threshold_met {
                "confidence threshold met"
            } else {
                "confidence threshold not met — deny"
            },
            outcome: if final_valid && threshold_met {
                EntryOutcome::Permitted
            } else {
                EntryOutcome::Denied
            },
        });

        Verdict {
            valid: final_valid && threshold_met,
            confidence,
            paradigms_evaluated: active_paradigms,
            trace,
            cross_validation,
            #[cfg(feature = "alloc")]
            formal_notation: synthesize_notation(&verdicts),
            #[cfg(feature = "alloc")]
            citations: collect_citations(&verdicts),
        }
    }
}

#[cfg(feature = "alloc")]
fn synthesize_notation(verdicts: &[Result<Verdict, urge_core::engine::EngineError>]) -> String {
    let notations: alloc::vec::Vec<String> = verdicts
        .iter()
        .filter_map(|v| v.as_ref().ok())
        .map(|v| v.formal_notation.clone())
        .filter(|s| !s.is_empty())
        .collect();
    if notations.is_empty() {
        String::from("(no notation)")
    } else {
        notations.join(" ∧ ")
    }
}

#[cfg(feature = "alloc")]
fn collect_citations(
    verdicts: &[Result<Verdict, urge_core::engine::EngineError>],
) -> alloc::vec::Vec<urge_core::decision::Citation> {
    verdicts
        .iter()
        .filter_map(|v| v.as_ref().ok())
        .flat_map(|v| v.citations.iter().cloned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use urge_core::engine::{ContextValue, EvalContext};

    fn empty_ctx() -> EvalContext<'static> {
        EvalContext {
            slots: &[],
            logical_time: 0,
            depth_limit: 16,
        }
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn evaluate_simple_true() {
        let pipeline = GovernancePipeline::new(PipelineConfig::default());
        let ctx = empty_ctx();
        let verdict = pipeline.evaluate_str("true", &ctx);
        assert!(verdict.valid);
        assert!(verdict.confidence.0 > 0);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn evaluate_simple_false() {
        let pipeline = GovernancePipeline::new(PipelineConfig::default());
        let ctx = empty_ctx();
        let verdict = pipeline.evaluate_str("false", &ctx);
        assert!(!verdict.valid);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn evaluate_negation() {
        let pipeline = GovernancePipeline::new(PipelineConfig::default());
        let ctx = empty_ctx();
        let verdict = pipeline.evaluate_str("not false", &ctx);
        assert!(verdict.valid);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn evaluate_deontic_obligation() {
        let pipeline = GovernancePipeline::new(PipelineConfig::default());
        let slots = &[("consent:done", ContextValue::Bool(true))];
        let ctx = EvalContext {
            slots,
            logical_time: 0,
            depth_limit: 16,
        };
        // "must" keyword triggers deontic engine
        let verdict = pipeline.evaluate_str("must true", &ctx);
        // Deontic engine evaluates the inner expression.
        assert!(verdict.paradigms_evaluated.contains(Paradigm::Deontic));
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn healthcare_config_exhaustive() {
        let pipeline = GovernancePipeline::default_healthcare();
        assert!(pipeline.config.exhaustive_evaluation);
        assert!(pipeline.config.confidence_threshold > 128);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn trace_is_non_empty() {
        let pipeline = GovernancePipeline::new(PipelineConfig::default());
        let ctx = empty_ctx();
        let verdict = pipeline.evaluate_str("always true", &ctx);
        assert!(!verdict.trace.is_empty());
    }
}
