//! Healthcare consent governance example.
//!
//! Demonstrates:
//! 1. Registering a HIPAA-required consent obligation with a 24-hour deadline
//! 2. Checking PHI access under governance
//! 3. Advancing time and detecting violations
//! 4. Satisfying the obligation via an action event
//!
//! This maps directly to the healthcare ERP agentic scenario described in
//! the patent: obligation-driven governance over a clinical event stream.

use urge_runtime::healthcare::HealthcareGovernor;

fn main() {
    println!("=== URGE Healthcare Consent Governance Demo ===\n");

    let mut gov = HealthcareGovernor::new();

    // ── Step 1: Patient admission triggers consent obligation ─────────────
    let _now_ns: u64 = 0; // t = 0
    let deadline_24h_ns: u64 = 86_400_000_000_000; // 24h in nanoseconds

    println!("[T+0] Patient P123 admitted. Registering consent obligation...");
    gov.require_consent("P123", "nurse_007", deadline_24h_ns);

    let stats = gov.stats();
    println!(
        "  Active obligations: {}, Violated: {}",
        stats.active_obligations, stats.violated_obligations
    );

    // ── Step 2: PHI access check at T+1h ─────────────────────────────────
    println!("\n[T+1h] nurse_007 requesting PHI access for P123...");
    let result = gov.check_phi_access(
        "nurse_007",
        "P123",
        true, // is_authorized
        true, // is_authenticated
        true, // audit_active
    );
    match result {
        Ok(_) => println!("  ✓ PHI access PERMITTED"),
        Err(e) => println!("  ✗ PHI access DENIED: {}", e),
    }

    // ── Step 3: Advance time past deadline without consent ────────────────
    let past_deadline_ns: u64 = deadline_24h_ns + 1_000_000_000; // 24h + 1s
    println!("\n[T+24h+1s] Advancing time past consent deadline...");
    let violations = gov.tick(past_deadline_ns);

    if violations.is_empty() {
        println!("  No violations detected.");
    } else {
        for v in &violations {
            println!("  ✗ VIOLATION: {:?}", v);
        }
    }

    let stats = gov.stats();
    println!(
        "  Active obligations: {}, Violated: {}",
        stats.active_obligations, stats.violated_obligations
    );

    // ── Step 4: Show audit log ────────────────────────────────────────────
    println!("\n--- Audit Log Summary ---");
    let log = gov.audit_log();
    println!(
        "  Total entries: {}, Permitted: {}, Denied: {}",
        log.total_entries(),
        log.permitted_count(),
        log.denied_count(),
    );

    println!("\n=== Demo complete ===");
    println!("\nThis trace is the compliance audit trail. Every decision is");
    println!("anchored to a formal logic evaluation with a full paradigm trace.");
    println!("See verdict.formal_notation and verdict.trace for full detail.");
}
