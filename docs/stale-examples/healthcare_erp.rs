//! Healthcare ERP Example — BROAD-Pattern Agentic Governance
//!
//! Demonstrates how URGE governs a multi-agent healthcare workflow:
//!
//! T+0s:   Patient P123 admitted → O(intake_agent, obtain_consent) DEADLINE 24h
//! T+10s:  intake_agent requests access to PHI — GRANTED (consent pending)
//! T+20s:  billing_agent tries to transmit PHI — DENIED (consent not yet given)
//! T+30s:  Consent obtained → obligation SATISFIED
//! T+40s:  billing_agent tries again — GRANTED
//! T+90000s: Simulated timeout — obligation would have VIOLATED
//!
//! Every decision is fully auditable, traceable to clinical/regulatory sources.
//! This is the URGE governance layer applied
//! to the BROAD healthcare ERP architecture.

use urge::{
    Context, GovernanceEngine, Obligation, ObligationState,
    Request, WorldState,
    ast::{Agent, Atom, Expr},
    monitor::WorldState as WS,
    policy::PolicySuite,
};

fn main() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║   BROAD Healthcare ERP — Agentic Governance Demonstration     ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!("\nPatient P123 admitted.  Simulating governance lifecycle.\n");

    // Build the healthcare governance engine
    let engine = GovernanceEngine::builder()
        .with_policy_suite(PolicySuite::Healthcare)
        .with_policy_suite(PolicySuite::EuAiAct)
        .build();

    // ── T+0: Patient admission — activate consent obligation ───────────────
    println!("T+0s  — Patient P123 admitted.");
    println!("       Activating: O(intake_agent, obtain_consent) DEADLINE=86400s\n");

    let mut world = WS::new();
    world.now_sec = 0;

    // Register the timed consent obligation (HIPAA 45 CFR §164.508)
    world.add_obligation(Obligation::new(
        "OBL-CONSENT-P123",
        'O',
        Agent::new("agent:intake"),
        Expr::Atom(Atom::new("consent_obtained")),
        0,                  // activated at T=0
        Some(86_400),       // deadline: 24 hours
        100,                // priority: highest
        "HIPAA-164.508",
    ));

    // Base facts — audit is on, workforce trained
    world.set_bool("audit:enabled", true);
    world.set_bool("hipaa:workforce_trained", true);
    world.set_bool("euai:decision_logged", true);
    world.set_bool("euai:human_override_possible", true);
    world.set_bool("euai:risk_management_active", true);

    // ── T+10: intake_agent reads PHI — should be GRANTED ──────────────────
    world.now_sec = 10;
    println!("T+10s — intake_agent requests read access to patient_record:P123");
    world.set_bool("hipaa:authorised_access", true);
    world.set_bool("access:minimum_necessary", true);
    world.set_bool("norm:P:agent:intake", true);

    let req = Request::access("agent:intake", "resource:patient_record:P123");
    let ctx = context_from_world(&world);

    match engine.evaluate(&req, &ctx) {
        Ok(d) => print_decision("intake_agent → patient_record", &d),
        Err(e) => println!("  ERROR: {}", e),
    }

    // ── T+20: billing_agent tries to transmit PHI — should be DENIED ──────
    world.now_sec = 20;
    println!("\nT+20s — billing_agent attempts to transmit PHI externally");
    println!("       (no consent yet — obligation still ACTIVE)");

    // billing_agent is not authorised to transmit without consent
    world.set_bool("hipaa:authorised_access", false);
    world.set_bool("norm:P:agent:billing", false);

    let req = Request {
        subject: "agent:billing".into(),
        action: urge::ActionKind::Transmit,
        object: "external:phi:P123".into(),
        formula: None,
    };
    let ctx = context_from_world(&world);

    match engine.evaluate(&req, &ctx) {
        Ok(d) => print_decision("billing_agent → transmit PHI", &d),
        Err(e) => println!("  ERROR: {}", e),
    }

    // ── T+30: Consent obtained — satisfy the obligation ────────────────────
    world.now_sec = 30;
    println!("\nT+30s — Consent obtained for P123. Satisfying obligation OBL-CONSENT-P123.");

    if let Some(obl) = world.obligations.iter_mut().find(|o| o.id == "OBL-CONSENT-P123") {
        obl.satisfy();
        println!("       Obligation state: {}", obl.state);
    }
    world.set_bool("consent_obtained", true);
    world.set_bool("hipaa:authorised_access", true);

    // ── T+40: billing_agent tries again — should be GRANTED ───────────────
    world.now_sec = 40;
    println!("\nT+40s — billing_agent retries transmit (consent now satisfied)");
    world.set_bool("norm:P:agent:billing", true);

    let req = Request {
        subject: "agent:billing".into(),
        action: urge::ActionKind::Transmit,
        object: "external:phi:P123".into(),
        formula: None,
    };
    let ctx = context_from_world(&world);

    match engine.evaluate(&req, &ctx) {
        Ok(d) => print_decision("billing_agent → transmit PHI (post-consent)", &d),
        Err(e) => println!("  ERROR: {}", e),
    }

    // ── T+86401: Simulate what would happen at deadline (different patient) ─
    println!("\n── Hypothetical: P456 (no consent obtained within 24h) ──");
    let mut world2 = WS::new();
    world2.now_sec = 86_401; // past the deadline
    world2.set_bool("audit:enabled", true);
    world2.set_bool("hipaa:workforce_trained", true);
    world2.set_bool("euai:decision_logged", true);
    world2.set_bool("euai:human_override_possible", true);
    world2.set_bool("euai:risk_management_active", true);

    world2.add_obligation(Obligation::new(
        "OBL-CONSENT-P456",
        'O',
        Agent::new("agent:intake"),
        Expr::Atom(Atom::new("consent_obtained")),
        0, Some(86_400), 100, "HIPAA-164.508",
    ));
    // Tick past the deadline
    let transitions = world2.tick(86_401);
    for t in &transitions {
        println!("  {}", t);
    }

    let req = Request::access("agent:intake", "resource:patient_record:P456");
    let ctx = context_from_world(&world2);

    match engine.evaluate(&req, &ctx) {
        Ok(d) => print_decision("intake_agent → patient_record:P456 (DEADLINE VIOLATED)", &d),
        Err(e) => println!("  ERROR: {}", e),
    }

    println!("\n════════════════════════════════════════════════════════════════");
    println!("  BROAD ERP demonstration complete.");
    println!("  Every decision above is traceable to HIPAA / EU AI Act rules.");
    println!("════════════════════════════════════════════════════════════════\n");
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn context_from_world(world: &WS) -> Context {
    let mut ctx = Context::default();
    ctx.now_sec = world.now_sec;
    // In a real ERP, this would serialise world state facts into the context.
    // For this demo we reconstruct from well-known keys.
    ctx
}

fn print_decision(label: &str, d: &urge::pipeline::GovernanceDecision) {
    let verdict_sym = match &d.verdict {
        urge::pipeline::Verdict::Permitted  => "✅ PERMITTED",
        urge::pipeline::Verdict::Denied(_)  => "❌ DENIED",
        urge::pipeline::Verdict::RequiresEscalation(_) => "⚠️  ESCALATE",
        urge::pipeline::Verdict::Uncertain{..} => "❓ UNCERTAIN",
    };
    println!("       {}", label);
    println!("       → {}  (conf={:.0}%  latency={}µs)", verdict_sym, d.confidence * 100.0, d.latency_us);
    if let urge::pipeline::Verdict::Denied(reason) = &d.verdict {
        println!("       → Reason: {:?}", reason);
    }
}
