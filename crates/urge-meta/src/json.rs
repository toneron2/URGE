//! JSON in, JSON out, for callers that are not Rust: the browser demo (`urge-wasm`) and the
//! `urge-eval` command (`urge-cli`). One implementation, so the two cannot disagree.
//!
//! Input: an expression and a flat JSON object of facts (`{"authorized": true,
//! "battery_pct": 80}`); booleans, integers and floats. Output: the `Verdict` as JSON, plus
//! `because` on a deny (the [`Explanation`](crate::explain::Explanation): each clause, the
//! facts it read, and the clauses that decided the deny; `null` on a permit) and `version`.

use alloc::{borrow::ToOwned, boxed::Box, format, string::String, vec::Vec};

use urge_core::engine::{ContextValue, EvalContext};

use crate::{GovernancePipeline, PipelineConfig};

/// `(valid, json)`: `valid` is `None` when the input was malformed, and `json` is then
/// `{"error": "..."}`.
pub fn evaluate(expr: &str, facts_json: &str, config: PipelineConfig) -> (Option<bool>, String) {
    let parsed: serde_json::Value = match serde_json::from_str(facts_json) {
        Ok(v) => v,
        Err(e) => return (None, error(&format!("facts are not valid JSON: {e}"))),
    };
    let Some(obj) = parsed.as_object() else {
        return (None, error("facts must be a JSON object of slots"));
    };
    let mut slots: Vec<(&'static str, ContextValue)> = Vec::with_capacity(obj.len());
    for (key, value) in obj {
        let cv = match value {
            serde_json::Value::Bool(b) => ContextValue::Bool(*b),
            serde_json::Value::Number(n) if n.is_i64() => {
                ContextValue::Integer(n.as_i64().unwrap_or(0))
            }
            serde_json::Value::Number(n) => ContextValue::Float(n.as_f64().unwrap_or(0.0)),
            other => {
                return (
                    None,
                    error(&format!("fact '{key}' has unsupported type: {other}")),
                )
            }
        };
        slots.push((intern(key), cv));
    }
    let ctx = EvalContext {
        slots: &slots,
        logical_time: 0,
        depth_limit: 32,
    };
    let pipeline = GovernancePipeline::new(config);
    let verdict = pipeline.evaluate_str(expr, &ctx);
    // Serialized once and extended in place: a Value tree in between doubled the cost.
    let mut out = match serde_json::to_string(&verdict) {
        Ok(s) => s,
        Err(e) => return (None, error(&format!("verdict serialization failed: {e}"))),
    };
    // Only a deny is explained: it costs a second pass over the clauses, and a permit has no
    // deciding clause to name.
    let because = if verdict.valid {
        String::from("null")
    } else {
        serde_json::to_string(&pipeline.explain(expr, &ctx))
            .unwrap_or_else(|_| String::from("null"))
    };
    out.pop(); // the verdict object's closing brace
    out.push_str(&format!(
        ",\"because\":{because},\"version\":\"{}\"}}",
        env!("CARGO_PKG_VERSION")
    ));
    (Some(verdict.valid), out)
}

fn error(msg: &str) -> String {
    serde_json::json!({ "error": msg }).to_string()
}

/// `EvalContext` slot names are `&'static str` (a no_std design choice), so dynamic names
/// are interned: each distinct name is leaked once and reused for the life of the process.
/// Bounded by the number of distinct fact names a caller uses.
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
    use super::*;

    fn run(expr: &str, facts: &str) -> serde_json::Value {
        let (_, out) = evaluate(expr, facts, PipelineConfig::healthcare());
        serde_json::from_str(&out).unwrap()
    }

    #[test]
    fn or_after_a_prefix_operator_is_or() {
        let v = run("must a or b", r#"{"a": false, "b": true}"#);
        assert_eq!(v["valid"], true, "{v}");
        assert_eq!(v["formal_notation"], "(O(a)) ∨ (b)");
        let v = run("always a implies must b", r#"{"a": false, "b": false}"#);
        assert_eq!(v["valid"], true, "{v}");
    }

    #[test]
    fn a_clean_refusal_is_certain_and_names_its_clause() {
        let v = run("must device_registered and must hardware_matches_register and must bio_signature_fresh",
                    r#"{"device_registered": true, "hardware_matches_register": false, "bio_signature_fresh": true}"#);
        assert_eq!(v["valid"], false);
        assert_eq!(v["confidence"], 255, "{v}");
        let d = v["because"]["deciding"].as_array().unwrap();
        assert_eq!(d.len(), 1);
        assert_eq!(
            v["because"]["clauses"][d[0].as_u64().unwrap() as usize]["notation"],
            "O(hardware_matches_register)"
        );
    }

    #[test]
    fn must_not_prints_as_obliged_not() {
        let v = run("must_not hazard_flag", r#"{"hazard_flag": false}"#);
        assert_eq!(v["valid"], true);
        assert_eq!(v["formal_notation"], "O(¬hazard_flag)");
    }

    #[test]
    fn a_bound_is_printed_and_kept() {
        assert_eq!(
            run("eventually reply within 30", r#"{"reply": false}"#)["formal_notation"],
            "F≤30(reply)"
        );
        let v = run("next x within 3", r#"{"x": true}"#);
        assert_eq!(v["valid"], false, "a bound after next is not a bound: {v}");
    }

    #[test]
    fn never_means_never() {
        assert_eq!(run("never breach", r#"{"breach": false}"#)["valid"], true);
        assert_eq!(run("never breach", r#"{"breach": true}"#)["valid"], false);
    }

    #[test]
    fn an_unparsed_expression_is_denied_with_its_reason() {
        let v = run("agent must obtain_consent", "{}");
        assert_eq!(v["valid"], false);
        assert!(
            v["formal_notation"]
                .as_str()
                .unwrap()
                .starts_with("unparsed: unparsed input from 'must'"),
            "{v}"
        );
    }

    #[test]
    fn malformed_facts_are_an_error() {
        let (valid, out) = evaluate("true", "[1]", PipelineConfig::default());
        assert!(valid.is_none() && out.contains("error"));
    }
}
