//! wasm-bindgen wrapper for the URGE browser demo.
//!
//! Exposes one call: [`evaluate_str`] — a governance expression plus a JSON
//! context object in, the full serialized `Verdict` out. The demo page in
//! `docs/demo/` is the only intended consumer; the crate is `publish = false`.

use urge_core::engine::{ContextValue, EvalContext};
use urge_meta::{GovernancePipeline, PipelineConfig};
use wasm_bindgen::prelude::*;

/// Evaluate `expr` against `ctx_json`, a flat JSON object of slots
/// (`{"authorized": true, "battery_pct": 80}`). Returns the full `Verdict`
/// serialized as JSON, or `{"error": "..."}` on malformed input.
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
    let parsed: serde_json::Value = match serde_json::from_str(ctx_json) {
        Ok(v) => v,
        Err(e) => return err_json(&format!("context is not valid JSON: {e}")),
    };
    let obj = match parsed.as_object() {
        Some(o) => o,
        None => return err_json("context must be a JSON object of slots"),
    };

    let mut slots: Vec<(&'static str, ContextValue)> = Vec::with_capacity(obj.len());
    for (key, value) in obj {
        let cv = match value {
            serde_json::Value::Bool(b) => ContextValue::Bool(*b),
            serde_json::Value::Number(n) if n.is_i64() => {
                ContextValue::Integer(n.as_i64().unwrap_or(0))
            }
            serde_json::Value::Number(n) => ContextValue::Float(n.as_f64().unwrap_or(0.0)),
            other => return err_json(&format!("slot '{key}' has unsupported type: {other}")),
        };
        slots.push((intern(key), cv));
    }

    let ctx = EvalContext {
        slots: &slots,
        logical_time: 0,
        depth_limit: 32,
    };

    // Exhaustive config: every applicable paradigm certifies, as in the
    // README's agent-gate example.
    let pipeline = GovernancePipeline::new(PipelineConfig::healthcare());
    let verdict = pipeline.evaluate_str(expr, &ctx);

    serde_json::to_string(&verdict)
        .unwrap_or_else(|e| err_json(&format!("verdict serialization failed: {e}")))
}

fn err_json(msg: &str) -> String {
    serde_json::to_string(&serde_json::json!({ "error": msg })).unwrap_or_default()
}

/// `EvalContext` slot names are `&'static str` (a no_std design choice), so
/// dynamic names from the browser are interned: each distinct name is leaked
/// once and reused for the life of the page. Bounded by the number of
/// distinct identifiers the user types.
fn intern(s: &str) -> &'static str {
    use std::sync::Mutex;
    static POOL: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let mut pool = POOL.lock().expect("intern pool poisoned");
    if let Some(hit) = pool.iter().find(|k| **k == s) {
        return hit;
    }
    let leaked: &'static str = Box::leak(s.to_owned().into_boxed_str());
    pool.push(leaked);
    leaked
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
