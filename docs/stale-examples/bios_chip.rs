//! BIOS Chip Governance Example
//!
//! Demonstrates URGE operating as a firmware-level governance primitive —
//! the "BIOS-like capability running in a chip" described in the patent.
//!
//! In a real embedded deployment:
//! - This would compile with `--no-default-features --features alloc`
//!   (or `--features embedded` for heapless mode)
//! - The world state lives in SRAM or flash-backed NVM
//! - The Unicode dictionary is in ROM (read-only flash, ~15KB)
//! - Each engine is a ~5-8KB function
//! - Total footprint target: ~100KB
//!
//! This example runs in std mode for local demonstration but uses only
//! the Device policy suite and demonstrates sub-millisecond evaluation.

use urge::{
    Context, GovernanceEngine,
    policy::PolicySuite,
    Request, ActionKind,
};

fn main() {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║   URGE BIOS Chip — Embedded Governance Demonstration       ║");
    println!("║   (Device policy suite — maps to firmware deployment)      ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let engine = GovernanceEngine::builder()
        .with_policy_suite(PolicySuite::Device)
        .with_latency_budget_us(5_000)  // 5ms budget for embedded target
        .build();

    // ── Scenario 1: Routine sensor access, battery healthy ─────────────────
    chip_scenario(
        &engine,
        "1. Camera access — battery=85%, normal operation",
        Request { subject: "sensor_daemon".into(), action: ActionKind::Read, object: "device:camera".into(), formula: None },
        Context::default()
            .with_fact("device:battery_ok", true)
            .with_fact("device:camera_permitted", true)
            .with_fact("device:emergency_active", false)
            .with_fact("modal:necessary:device:emergency_active->device:all_permitted", false),
    );

    // ── Scenario 2: Camera denied — low battery ────────────────────────────
    chip_scenario(
        &engine,
        "2. Camera access — battery=8%, denied by power policy",
        Request { subject: "sensor_daemon".into(), action: ActionKind::Read, object: "device:camera".into(), formula: None },
        Context::default()
            .with_fact("device:battery_ok", false)
            .with_fact("device:camera_permitted", false)
            .with_fact("device:emergency_active", false),
    );

    // ── Scenario 3: Biometric data — transmit denied without consent ────────
    chip_scenario(
        &engine,
        "3. Biometric transmit — no user consent",
        Request { subject: "telemetry_agent".into(), action: ActionKind::Transmit, object: "device:biometric:fingerprint".into(), formula: None },
        Context::default()
            .with_fact("device:user_consent_given", false)
            .with_fact("device:transmit_biometric_without_consent", true), // fact: this IS happening without consent
    );

    // ── Scenario 4: Emergency override — all resources unlocked ────────────
    chip_scenario(
        &engine,
        "4. Emergency override — low battery BUT emergency flag active",
        Request { subject: "emergency_service".into(), action: ActionKind::Read, object: "device:camera".into(), formula: None },
        Context::default()
            .with_fact("device:battery_ok", false)
            .with_fact("device:emergency_active", true)
            .with_fact("device:all_permitted", true)
            .with_fact("modal:necessary:device:emergency_active->device:all_permitted", true),
    );

    // ── Scenario 5: Direct formula — modal necessity check ─────────────────
    chip_scenario(
        &engine,
        "5. System invariant check — □(audit_log_never_disabled)",
        Request::formula("system", "[](audit_log_never_disabled)"),
        Context::default()
            .with_fact("modal:necessary:audit_log_never_disabled", true),
    );

    println!("\nAll BIOS scenarios evaluated.");
    println!("Target latency for firmware deployment: <5,000µs per decision.");
    println!("Footprint target: <100KB (engines: ~40KB, dict: ~15KB, kernel: ~10KB)\n");
}

fn chip_scenario(engine: &GovernanceEngine, label: &str, req: Request, ctx: Context) {
    print!("  {:<55}", label);
    match engine.evaluate(&req, &ctx) {
        Ok(d) => {
            let sym = if d.verdict.is_permitted() { "✅" } else { "❌" };
            println!("{} {:?}  [{}µs]", sym, d.verdict, d.latency_us);
        }
        Err(e) => println!("ERR: {}", e),
    }
}
