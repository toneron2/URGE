//! wasm-bindgen wrapper for the URGE browser demo.
//!
//! Exposes one call: [`evaluate_str`] — a governance expression plus a JSON
//! context object in, the full serialized `Verdict` out. The demo page in
//! `docs/demo/` is the only intended consumer; the crate is `publish = false`.

use urge_meta::PipelineConfig;
use wasm_bindgen::prelude::*;

/// Evaluate `expr` against `ctx_json`, a flat JSON object of slots
/// (`{"authorized": true, "battery_pct": 80}`). Returns the full `Verdict`
/// serialized as JSON with `version` and, on a deny, `because` (each clause, the facts it
/// read, and the clauses that decided the deny), or `{"error": "..."}` on malformed input.
#[wasm_bindgen]
pub fn evaluate_str(expr: &str, ctx_json: &str) -> String {
    evaluate_impl(expr, ctx_json)
}

/// Crate version, for the demo footer.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").into()
}

fn evaluate_impl(expr: &str, ctx_json: &str) -> String {
    // Exhaustive config: every applicable paradigm certifies, as in the README's agent-gate
    // example. The JSON handling is urge-meta's (feature `json`), shared with `urge-eval`.
    urge_meta::json::evaluate(expr, ctx_json, PipelineConfig::healthcare()).1
}

#[cfg(test)]
mod tests {
    use super::evaluate_impl;

    #[test]
    fn permit_case_serializes() {
        let out = evaluate_impl(
            "must authorized and always audit_running",
            r#"{"authorized": true, "audit_running": true}"#,
        );
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["valid"], true, "verdict JSON: {out}");
        assert_eq!(v["formal_notation"], "O(authorized) ∧ G(audit_running)");
        assert!(v["trace"]["entries"].as_array().unwrap().len() > 5);
    }

    #[test]
    fn conflict_case_serializes() {
        let out = evaluate_impl(
            "must authorized and always audit_running",
            r#"{"authorized": true, "audit_running": false}"#,
        );
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["valid"], false);
        assert_eq!(v["cross_validation"]["consistent"], false);
    }

    #[test]
    fn bad_context_reports_error() {
        let out = evaluate_impl("true", "not json");
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v["error"].is_string());
    }
}
