# URGE — Universal Reasoning Governance Engine

> **Patent pending.** Core architecture covered by provisional patent application  
> *"Unicode Semantic Dictionary and Multi-Logic Processing Architecture for  
> Modular Reasoning Systems"* — TODOMODO.IO AGENCY LLC / Anthony R. Slosar

URGE is a **formal, deterministic governance layer** that can be embedded into
any system requiring auditable, reasoning-driven access control and policy
enforcement — from a BIOS-like chip on edge hardware to a cloud-scale agentic
healthcare ERP.

---

## The Problem

Every AI agent system today has the same gap: **capability without governance**.

```
Traditional policy engines:   Request → Evaluate → Allow/Deny   (stateless, single logic)
LLM alignment:                Behavioral, implicit, non-auditable, learned
URGE:                         Event → Multi-Paradigm Reasoning → Auditable Verdict
                              Deterministic · Formal · Edge-capable · Patent-pending
```

When a healthcare agent recommends a medication change, you must be able to answer:
*"Why did the system allow this?"* In most systems the answer is "the model said so."
In URGE the answer is a fully traced, formally notated reasoning chain — anchored
to the regulatory source that justified the decision.

---

## Architecture — Figure 26 Pipeline

The core of URGE is the **Adaptive Processing Workflow** described in Figure 26
of the provisional patent, implemented in `urge-meta`:

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
│ STAGE 6: CROSS-SYSTEM VALIDATION  ◄── KEY PATENT INNOVATION      │
│   Inter-paradigm consistency checking across all engine results  │
│   Contradiction detection: Modal vs Boolean, Temporal vs Deontic │
│   No equivalent exists in OPA, Drools, REGO, or any OSS system  │
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
| Fuzzy           | μ ⊓ ⊔  likely  probability             | Clinical thresholds, uncertainty     |
| Probabilistic   | P(·)  prior  likelihood                | Bayesian confidence                  |
| Paraconsistent  | Belnap 4-valued: T F Both Neither      | Contradiction without explosion      |

---

## Deployment Targets

### BIOS / Embedded Chip (`no_std`, zero allocation)

> **Status: experimental / work in progress.** The `std` and `alloc` tiers are
> fully functional and tested. The fully heap-free (`no_std` + no-`alloc`) tier
> does **not** yet compile — the recursive AST needs a heap-free (arena/index)
> representation, which is planned but not yet implemented. The numbers below are
> design targets, not yet a shipping capability.

```
Feature flags: (default — no std required)
Footprint:     ~30 KB Flash  (dict + engines + shallow AST + obligation table)
Latency:       <5ms on Cortex-M4,  <1µs on x86
Heap:          Zero allocation in the access-control hot path
Comparison:    Python interpreter ~8 MB  →  URGE ~1/250th the size
```

### Agentic Healthcare ERP (`std`, `healthcare`)

```
Feature flags: std, healthcare
Config:        PipelineConfig::healthcare()  (exhaustive, all 7 paradigms always run)
Latency:       <50ms full multi-paradigm evaluation
Compliance:    HIPAA §164.312, EU AI Act, APA clinical CPGs
Audit:         Every decision logged with formal Unicode notation + regulatory citations
```

### Agent Governance Gate (`std`)

```
Pattern:  Wrap every autonomous agent action in a governance call before execution
Output:   Permit/Deny with full logic trace for post-hoc audit
Benefit:  Deontic obligation lifecycle tracking, not just per-request policy checks
```

---

## Quick Start

### String expression evaluation

```rust
use urge_meta::{GovernancePipeline, PipelineConfig};
use urge_core::engine::{ContextValue, EvalContext};

let pipeline = GovernancePipeline::new(PipelineConfig::healthcare());

let slots = &[
    ("authorized",   ContextValue::Bool(true)),
    ("audit_active", ContextValue::Bool(true)),
];
let ctx = EvalContext { slots, logical_time: 0, depth_limit: 16 };

let verdict = pipeline.evaluate_str("must authorized and always audit_active", &ctx);

assert!(verdict.valid);
println!("Formal: {}", verdict.formal_notation);
// → "O(authorized) ∧ G(audit_active)"
println!("Confidence: {:.0}%", verdict.confidence.as_f32() * 100.0);
for entry in &verdict.trace.entries {
    println!("  [{:?}] {}", entry.stage, entry.description);
}
```

### Healthcare — HIPAA consent obligation

```rust
use urge_runtime::healthcare::HealthcareGovernor;

let mut gov = HealthcareGovernor::new();

// Register HIPAA obligation: obtain informed consent within 24 hours of admission.
gov.require_consent("P123", "nurse_007", 86_400_000_000_000); // 24h in nanoseconds

// Check PHI access — result anchored to HIPAA §164.312(a).
gov.check_phi_access("nurse_007", "P123", true, true, true)?;

// Advance time — automatically detects deadline violations.
let violations = gov.tick(now_ns());
for v in &violations {
    eprintln!("VIOLATION: {:?}", v);
}
```

### Embedded BIOS access control

> **Experimental.** Runs today on `std`/`alloc` targets; the fully heap-free
> `no_std` build is still WIP (see Deployment Targets above).

```rust
use urge_runtime::embedded::BiosGovernor;

let gov = BiosGovernor::new();
// Zero-allocation, deterministic, <5ms on embedded targets.
let permitted = gov.check_access("camera", "app.health", battery_pct);
```

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
│   │   └── validator.rs       Cross-system validation  ← key innovation
│   │
│   ├── urge-monitor/          Continuous governance over event streams
│   │   ├── obligation.rs      Lifecycle: PENDING→ACTIVE→SATISFIED/VIOLATED
│   │   ├── temporal.rs        LTL runtime monitors (G, F, U)
│   │   └── engine.rs          GovernanceMonitor — event-driven orchestration
│   │
│   └── urge-runtime/          std-tier public API
│       ├── healthcare.rs      HealthcareGovernor (HIPAA + clinical CPGs)
│       ├── embedded.rs        BiosGovernor (no_std capability access control)
│       └── audit.rs           Append-only audit log, NDJSON export
│
├── examples/
│   ├── healthcare_consent.rs  HIPAA consent obligation lifecycle demo
│   ├── bios_access.rs         BIOS capability governance demo
│   └── agent_gate.rs          Multi-agent governance gate demo
│
├── tests/
│   └── integration.rs         End-to-end pipeline tests (19 tests)
│
└── docs/
    └── ARCHITECTURE.md        Shell proof, memory model, compliance traceability
```

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

## Why This Beats Traditional Policy Engines

| Capability                        | OPA / Rego | Drools  | URGE  |
|-----------------------------------|-----------|---------|-------|
| Multi-paradigm (8+ logics)        | ✗         | ✗       | **✓** |
| Cross-paradigm validation         | ✗         | ✗       | **✓** |
| Deontic obligation lifecycle      | ✗         | partial | **✓** |
| Temporal LTL monitoring           | ✗         | partial | **✓** |
| Paraconsistent contradiction mgmt | ✗         | ✗       | **✓** |
| Embedded / no_std capable         | ✗         | ✗       | WIP¹  |
| Full logic trace + audit          | partial   | partial | **✓** |
| Formal Unicode notation output    | ✗         | ✗       | **✓** |
| Regulatory citation anchoring     | ✗         | ✗       | **✓** |
| <50ms healthcare decisions        | ✓         | ✓       | **✓** |

¹ Embedded `no_std` support is architected but **not yet functional** — the
heap-free AST representation is still WIP. The `std`/`alloc` tiers are complete.

The critical missing feature in every existing system: **cross-system validation.**
OPA evaluates rules independently. URGE checks that results across paradigms are
mutually consistent — and flags contradictions before producing a verdict. When
a Temporal deadline violation conflicts with an active Deontic obligation, that
conflict is detected, traced, and reported rather than silently resolved.

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

## Intellectual Property

**Patent pending.** This architecture is covered by a provisional patent
application filed by TODOMODO.IO AGENCY LLC (Anthony R. Slosar).

The reference implementation in this repository is **MIT licensed**. The core
architectural innovations — Unicode Semantic Dictionary, multi-paradigm
synthesis with cross-system validation, deontic obligation lifecycle management
combined with temporal LTL monitoring — are **patent-pending** and require
a separate license for commercial use.

Contact: https://t.me/toneron2  
See [`LICENSE`](LICENSE) for full terms including the patent notice.

---

## License

MIT — © 2025 TODOMODO.IO AGENCY LLC / Anthony R. Slosar

---

*The system that has no governance is the system that cannot be trusted.*  
*— BROAD architecture principle*
