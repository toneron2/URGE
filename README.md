# URGE — Universal Reasoning Governance Engine

[![CI](https://github.com/toneron2/URGE/actions/workflows/ci.yml/badge.svg)](https://github.com/toneron2/URGE/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust 1.70+](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
![Status: v0.1.0](https://img.shields.io/badge/status-v0.1.0-brightgreen.svg)

**LLM agents can act — but they can't prove why an action was allowed.**
URGE is a deterministic governance gate: wrap any agent action in a formal
policy check and get back an auditable verdict with a full reasoning trace.

Before your agent executes an action, it asks URGE. URGE parses a governance
expression (`must authorized and always audit_running`), routes it through up
to seven formal logic engines — deontic, temporal/LTL, epistemic, modal, fuzzy,
paraconsistent, boolean — cross-checks the engines' results against each other,
and returns a verdict: permitted or denied, with a confidence score, the formal
notation (`O(authorized) ∧ G(audit_running)`), and a stage-by-stage trace you
can store as the audit record. No LLM in the loop; the same input always
produces the same verdict.

---

## The Problem

Every AI agent system today has the same gap: **capability without governance**.

```
Traditional policy engines:   Request → Evaluate → Allow/Deny   (stateless, single logic)
LLM alignment:                Behavioral, implicit, non-auditable, learned
URGE:                         Event → Multi-Paradigm Reasoning → Auditable Verdict
                              Deterministic · Formal · Embeddable · Auditable
```

When an agent proposes an action — call a payment API, modify a patient record,
send an email on your behalf — you must be able to answer *"why did the system
allow this?"* In most stacks the answer is "the model seemed aligned." In URGE
the answer is a fully traced, formally notated reasoning chain.

---

## Quick Start — gate an agent action

```rust
use urge_core::engine::{ContextValue, EvalContext};
use urge_meta::{GovernancePipeline, PipelineConfig};

// Exhaustive config: every applicable paradigm runs on every evaluation.
let pipeline = GovernancePipeline::new(PipelineConfig::healthcare());

// The world as the gate sees it right now.
let slots = &[
    ("authorized",    ContextValue::Bool(true)),
    ("audit_running", ContextValue::Bool(false)), // audit trail is down
];
let ctx = EvalContext { slots, logical_time: 0, depth_limit: 16 };

// Policy: the action requires authorization (deontic obligation)
// AND a continuously running audit trail (temporal G).
let verdict = pipeline.evaluate_str("must authorized and always audit_running", &ctx);

assert!(!verdict.valid); // DENIED — and URGE can prove why:
println!("Formal:     {}", verdict.formal_notation); // O(authorized) ∧ G(audit_running)
println!("Confidence: {:.0}%", verdict.confidence.as_f32() * 100.0);
for entry in &verdict.trace.entries {
    println!("  [{:?}] {}", entry.stage, entry.description);
}
```

The full multi-agent version (four agents, mixed verdicts, trace printout):

```
cargo run -p urge-runtime --example agent_gate
```

Until the crates are on crates.io, depend on the git repo:

```toml
[dependencies]
urge-meta = { git = "https://github.com/toneron2/URGE" }
urge-core = { git = "https://github.com/toneron2/URGE" }
```

For obligations that live *across* requests (deadlines, escalation,
violation events), see `urge-monitor` and the obligation lifecycle below.

---

## Architecture — Figure 26 Pipeline

The core of URGE is the **Adaptive Processing Workflow** — the Figure 26
pipeline, named for the diagram in the original design document — implemented
in `urge-meta`:

```
INPUT (governance expression or event)
  │
  ▼
┌──────────────────────────────────────────────────────────────────┐
│ STAGE 1: TOKENIZATION                                            │
│   Unicode Semantic Dictionary (300+ operators, 15+ paradigms)    │
│   Every symbol classified before any evaluation                  │
│   [ Shell-proof heritage: grep -oP | awk | classify pipeline ]   │
├──────────────────────────────────────────────────────────────────┤
│ STAGE 2: PARADIGM DETECTION                                      │
│   Token classes → ParadigmSet (compact u16 bitset)               │
│   Determines which engines activate — deterministically          │
├──────────────────────────────────────────────────────────────────┤
│ STAGE 3: AST CONSTRUCTION                                        │
│   Pratt parser → typed, paradigm-annotated expression tree       │
├──────────────────────────────────────────────────────────────────┤
│ STAGE 4: ENGINE ROUTING (SWITCH)                                 │
│   Deterministic dispatch — no learned gating, no inference       │
│   Priority: Deontic > Temporal > Epistemic > Modal > Fuzzy >     │
│             Paraconsistent > Boolean                             │
├──────────────────────────────────────────────────────────────────┤
│ STAGE 5: ENGINE EVALUATION                                       │
│   Each selected engine evaluates its AST fragment                │
│   Returns partial Verdict: result + confidence + trace           │
├──────────────────────────────────────────────────────────────────┤
│ STAGE 6: CROSS-SYSTEM VALIDATION  ◄── DIFFERENTIATOR             │
│   Inter-paradigm consistency checking across all engine results  │
│   Contradiction detection: Modal vs Boolean, Temporal vs Deontic │
│   Conflicts are reported and traced, never silently resolved     │
├──────────────────────────────────────────────────────────────────┤
│ STAGE 7: VERDICT SYNTHESIS                                       │
│   Aggregate confidence, merge traces, apply threshold            │
│   Emit: valid + confidence + formal_notation + citations         │
└──────────────────────────────────────────────────────────────────┘
  │
  ▼
VERDICT { valid, confidence, paradigms_evaluated, trace, formal_notation, citations }
```

Every stage emits `TraceEntry` records. The complete trace IS the compliance audit trail.

---

## Supported Logic Paradigms

| Paradigm        | Key Operators                          | Governance Role                      |
|-----------------|----------------------------------------|--------------------------------------|
| Boolean         | ∧ ∨ ¬ → ↔ ⊕ ⊤ ⊥                      | Base layer — all systems             |
| Modal (S5)      | □ ◇  necessarily  possibly             | Access across system states          |
| Epistemic       | K B C  knows  believes                 | Agent knowledge verification         |
| **Deontic**     | **O P F  must  may  must_not**         | **Obligations, permissions**         |
| **Temporal/LTL**| **G F X U  always  eventually  until** | **Deadline enforcement**             |
| Fuzzy           | μ ⊓ ⊔  likely  probability             | Thresholds, uncertainty              |
| Paraconsistent  | Belnap 4-valued: T F Both Neither      | Contradiction without explosion      |
| Probabilistic¹  | P(·)  prior  likelihood                | Bayesian confidence (planned)        |

¹ Probabilistic operators are classified by the Unicode Semantic Dictionary,
but no dedicated probabilistic engine ships yet — the seven engines above are
implemented and tested.

---

## How URGE Compares to Traditional Policy Engines

| Capability                        | OPA / Rego | Drools  | URGE  |
|-----------------------------------|-----------|---------|-------|
| Multi-paradigm (7 logics)         | ✗         | ✗       | **✓** |
| Cross-paradigm validation²        | ✗         | ✗       | **✓** |
| Deontic obligation lifecycle      | ✗         | partial | **✓** |
| Temporal LTL monitoring           | ✗         | partial | **✓** |
| Paraconsistent contradiction mgmt | ✗         | ✗       | **✓** |
| Embedded / no_std capable         | ✗         | ✗       | WIP³  |
| Full logic trace + audit          | partial   | partial | **✓** |
| Formal Unicode notation output    | ✗         | ✗       | **✓** |
| Regulatory citation anchoring     | ✗         | ✗       | **✓** |
| Maturity, ecosystem, tooling      | **✓✓**    | **✓✓**  | v0.1  |

² Reproducible side-by-side in [`examples/comparison_opa/`](examples/comparison_opa/) —
the same policy in Rego and in URGE, showing what each system reports.

³ Embedded `no_std` support is architected but **not yet functional** — the
heap-free AST representation is still WIP. The `std`/`alloc` tiers are complete.

OPA and Drools evaluate rules within a single logic — and do it with mature
tooling, a large ecosystem, and years of production hardening that URGE does
not have. What URGE adds is **cross-paradigm evaluation**: it evaluates one
expression under several formal logics at once, then cross-checks the results
*between* paradigms and reports contradictions instead of silently resolving
them. When a temporal deadline violation conflicts with an active deontic
obligation, that conflict is detected, traced, and reported in the verdict.

---

## Obligation Lifecycle — Continuous Governance

Unlike request/response policy engines, URGE tracks obligations over time:

```
T+0:    patient_admission(P123) fires
        → CREATE  O(obtain_consent, P123)  deadline=T+24h
        → CREATE  G(audit_all_access, P123)  [permanent]

T+1h:   nurse_007 requests PHI access
        → EVALUATE  must authorized and always audit_active
        → PERMITTED  (logged to audit trail with §164.312 citation)

T+25h:  time_tick fires (deadline exceeded, no consent received)
        → O(obtain_consent, P123)  PENDING → VIOLATED
        → EMIT  ViolationEvent { patient: P123, obligation: consent, deadline: T+24h }
        → ESCALATE  to supervisor

T+26h:  nurse_007 performs consent intake
        → ActionCompleted(nurse_007, obtain_consent)
        [Obligation already VIOLATED — irreversible. New obligation needed.]
```

This is implemented in `urge-monitor::obligation::ObligationManager`.

---

## The Shell Proof

The original logic engine was proved viable as a bash regex pipeline.
The key insight: paradigm classification is **deterministic from the token stream**.

```bash
echo "agent must obtain_consent before deadline" \
  | tr ' ' '\n' \
  | while read token; do
      case "$token" in
        must|should|ought)     echo "DEONTIC:OBLIGATORY $token" ;;
        may|permitted|allowed) echo "DEONTIC:PERMITTED  $token" ;;
        must_not|forbidden)    echo "DEONTIC:FORBIDDEN  $token" ;;
        always|globally)       echo "TEMPORAL:G  $token" ;;
        eventually|finally)    echo "TEMPORAL:F  $token" ;;
        until|before|deadline) echo "TEMPORAL:U  $token" ;;
        and) echo "BOOLEAN:AND $token" ;;
        or)  echo "BOOLEAN:OR  $token" ;;
        not) echo "BOOLEAN:NOT $token" ;;
        *)   echo "IDENT       $token" ;;
      esac
    done | sort -k1 -u
# Output:
#   DEONTIC:OBLIGATORY must
#   TEMPORAL:U         before
#   TEMPORAL:U         deadline
# → Active paradigms: {Deontic, Temporal}
# → Route to: DeonticEngine + TemporalEngine
```

The Rust `UnicodeSemanticDictionary` does this in **<1µs** via a static
compile-time table vs. the shell's ~10ms process-startup overhead. The
architecture is identical. The governance insight is the same: you do not
need inference to decide that "must" is deontic.

---

## Crate Structure

```
urge/                          Cargo workspace
├── crates/
│   ├── urge-core/             no_std kernel
│   │   ├── symbol.rs          Unicode Semantic Dictionary (300+ operators)
│   │   ├── ast.rs             Multi-paradigm AST (heap or heapless)
│   │   ├── engine.rs          LogicEngine trait + Paradigm enum
│   │   └── decision.rs        Verdict, LogicTrace, Confidence, CrossValidation
│   │
│   ├── urge-engines/          Seven logic engine implementations
│   │   ├── boolean.rs         Propositional logic (base)
│   │   ├── deontic.rs         SDL: O / P / F obligations  ← governance heart
│   │   ├── temporal.rs        LTL: G / F / X / U / R
│   │   ├── modal.rs           S5: □ / ◇
│   │   ├── epistemic.rs       K / B / C multi-agent knowledge
│   │   ├── fuzzy.rs           Zadeh min/max fuzzy logic
│   │   └── paraconsistent.rs  Belnap 4-valued (contradiction without explosion)
│   │
│   ├── urge-meta/             Figure 26 pipeline implementation
│   │   ├── pipeline.rs        GovernancePipeline — all 7 stages
│   │   ├── tokenizer.rs       Unicode + keyword scanner
│   │   ├── parser.rs          Pratt AST builder
│   │   ├── router.rs          Deterministic engine SWITCH
│   │   └── validator.rs       Cross-system validation  ← key differentiator
│   │
│   ├── urge-monitor/          Continuous governance over event streams
│   │   ├── obligation.rs      Lifecycle: PENDING→ACTIVE→SATISFIED/VIOLATED
│   │   ├── temporal.rs        LTL runtime monitors (G, F, U)
│   │   └── engine.rs          GovernanceMonitor — event-driven orchestration
│   │
│   └── urge-runtime/          std-tier public API
│       ├── healthcare.rs      HealthcareGovernor (HIPAA + clinical CPGs)
│       ├── embedded.rs        BiosGovernor (capability access control)
│       ├── audit.rs           Append-only audit log, NDJSON export
│       └── examples/
│           ├── agent_gate.rs          Multi-agent governance gate demo
│           ├── healthcare_consent.rs  HIPAA consent obligation lifecycle demo
│           └── bios_access.rs         BIOS capability governance demo
│
└── docs/
    ├── ARCHITECTURE.md        Shell proof, memory model, compliance traceability
    └── USE_CASES.md           Healthcare ERP + embedded/BIOS deployment targets
```

19 end-to-end integration tests live in `crates/urge-runtime/tests/integration.rs`.

---

## Other Deployment Targets

The agent gate is the primary use case, but the same pipeline serves:

- **Agentic healthcare ERP** (`std` + `healthcare` feature) — obligation-driven
  HIPAA/clinical governance over an event stream, every decision anchored to a
  regulatory citation.
- **Embedded / BIOS-level governance** (`no_std`) — a deterministic "governance
  chip" mediating device capability access. **Experimental:** the fully
  heap-free (`no_std` + no-`alloc`) tier does **not compile yet** — the
  recursive AST still needs an arena/index representation. The `std`/`alloc`
  tiers are complete and tested.

Details, code snippets, and honest status notes for both:
[`docs/USE_CASES.md`](docs/USE_CASES.md).

---

## Status & Roadmap

v0.1.0 — the `std`/`alloc` tiers are complete: 35 tests passing, clippy- and
rustfmt-clean, CI on every push. The heap-free embedded tier and a dedicated
probabilistic engine are the two big open items. Full status table and
priorities: [`ROADMAP.md`](ROADMAP.md).

---

## Community & Contact

- Bug reports and feature requests: [GitHub Issues](https://github.com/toneron2/URGE/issues)
- Questions and design discussion: [GitHub Discussions](https://github.com/toneron2/URGE/discussions)
- Email: <anthonyslosar@gmail.com>

Contributions welcome — see [`CONTRIBUTING.md`](CONTRIBUTING.md).

---

## License

Apache-2.0 — © 2025 TODOMODO.IO AGENCY LLC / Anthony R. Slosar

Apache 2.0 includes an explicit patent grant: you can use, modify, and ship
URGE commercially without asking.

---

*The system that has no governance is the system that cannot be trusted.*  
*— BROAD architecture principle*
