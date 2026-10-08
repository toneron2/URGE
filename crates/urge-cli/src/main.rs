//! `urge-eval`: one governance decision from the command line.
//!
//! ```text
//! echo '{"expr": "must authorized and must_not breach", "facts": {"authorized": true}}' | urge-eval
//! ```
//!
//! Reads one JSON object from standard input: `expr` (the expression), `facts` (a flat object
//! of booleans; a fact not supplied reads as false, and so does a number for now) and optionally `config`
//! (`"healthcare"`, the default and the browser demo's: every paradigm certifies and the
//! confidence threshold is 0.80; `"standard"`: 0.50; `"embedded"`: 0.20). Writes the verdict
//! as one line of JSON; on a deny, `because` names the clauses that decided it.
//!
//! Exit status: 0 permitted, 1 denied, 2 malformed input.

use std::io::Read;

use urge_meta::PipelineConfig;

fn main() {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("urge-eval {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let mut input = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut input) {
        fail(&format!("cannot read standard input: {e}"));
    }
    let req: serde_json::Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(e) => fail(&format!("input is not valid JSON: {e}")),
    };
    let Some(expr) = req["expr"].as_str() else {
        fail("input needs \"expr\": the expression, a string")
    };
    let facts = if req["facts"].is_null() {
        "{}".to_string()
    } else {
        req["facts"].to_string()
    };
    let config = match req["config"].as_str().unwrap_or("healthcare") {
        "healthcare" => PipelineConfig::healthcare(),
        "standard" => PipelineConfig::default(),
        "embedded" => PipelineConfig::embedded(),
        other => fail(&format!(
            "unknown config \"{other}\": healthcare, standard or embedded"
        )),
    };
    let (valid, out) = urge_meta::json::evaluate(expr, &facts, config);
    println!("{out}");
    std::process::exit(match valid {
        Some(true) => 0,
        Some(false) => 1,
        None => 2,
    });
}

fn fail(msg: &str) -> ! {
    println!("{}", serde_json::json!({ "error": msg }));
    std::process::exit(2);
}
