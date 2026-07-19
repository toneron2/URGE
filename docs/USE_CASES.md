# URGE Use Cases — Beyond the Agent Gate

The README leads with URGE as a governance gate for AI agents. The same
pipeline was designed from the start for two further deployment targets.
Both are described here with their honest status.

---

## Agentic Healthcare ERP (`std`, `healthcare`)

Obligation-driven governance over a clinical event stream: every PHI access,
prescription, and consent workflow gated by formal logic and logged with a
regulatory citation.

```
Feature flags: std, healthcare
Config:        PipelineConfig::healthcare()  (exhaustive, all paradigms always run)
Latency:       <50ms full multi-paradigm evaluation
Compliance:    HIPAA §164.312, EU AI Act, APA clinical CPGs
Audit:         Every decision logged with formal Unicode notation + regulatory citations
```

### HIPAA consent obligation

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

Runnable demo:

```
cargo run -p urge-runtime --example healthcare_consent
```

**Status note:** the `HealthcareGovernor` API is functional and tested on the
`std` tier. The compliance mappings (HIPAA sections, CPG citations) are
illustrative anchors, not certified compliance tooling — treat them as a
pattern for wiring your own regulatory citations into the audit trail.

---

## BIOS / Embedded Chip (`no_std`, zero allocation)

> **Status: experimental / work in progress.** The `std` and `alloc` tiers are
> fully functional and tested. The fully heap-free (`no_std` + no-`alloc`) tier
> does **not** yet compile — the recursive AST needs a heap-free (arena/index)
> representation, which is planned but not yet implemented. The numbers below are
> design targets, not yet a shipping capability.

```
Feature flags: (default — no std required)
Footprint:     ~30 KB Flash  (dict + engines + shallow AST + obligation table)   [target]
Latency:       <5ms on Cortex-M4,  <1µs on x86                                   [target]
Heap:          Zero allocation in the access-control hot path                    [target]
Comparison:    Python interpreter ~8 MB  →  URGE ~1/250th the size               [target]
```

### BIOS access control

> **Experimental.** Runs today on `std`/`alloc` targets; the fully heap-free
> `no_std` build is still WIP (see status note above).

```rust
use urge_runtime::embedded::BiosGovernor;

let gov = BiosGovernor::new();
// Deterministic capability gating — heap-free operation is the design target.
let permitted = gov.check_access("camera", "app.health", battery_pct);
```

Runnable demo:

```
cargo run -p urge-runtime --example bios_access
```

The blocker and the plan (arena/index AST) are described in
[`ROADMAP.md`](../ROADMAP.md) — it is the top roadmap item.

---

## Older worked examples

Two pre-rewrite examples (`bios_chip.rs`, `healthcare_erp.rs`) are quarantined
in [`docs/stale-examples/`](stale-examples/) — they show the intended end-state
API surface but do not compile against the current crates. Porting them is a
roadmap item.
