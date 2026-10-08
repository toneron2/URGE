# URGE Architecture Deep Dive

## Origins and the Shell Proof

The URGE logic engine was first proved viable using shell tooling. The key
insight: paradigm classification is deterministic from the token stream, and
token classification is a pure dictionary lookup.

```bash
# The original proof: classify a governance expression in bash
EXPR="agent must obtain_consent before deadline"

echo "$EXPR" \
  | tr ' ' '\n' \
  | while read token; do
      case "$token" in
        must|should|ought|obligatory) echo "DEONTIC:OBLIGATORY $token" ;;
        may|permitted|allowed)        echo "DEONTIC:PERMITTED  $token" ;;
        must_not|forbidden|prohibited) echo "DEONTIC:FORBIDDEN $token" ;;
        always|globally)              echo "TEMPORAL:G  $token" ;;
        eventually|finally)           echo "TEMPORAL:F  $token" ;;
        until|before|deadline)        echo "TEMPORAL:U  $token" ;;
        and|∧)                        echo "BOOLEAN:AND $token" ;;
        or|∨)                         echo "BOOLEAN:OR  $token" ;;
        not|¬)                        echo "BOOLEAN:NOT $token" ;;
        *)                            echo "IDENT       $token" ;;
      esac
    done \
  | sort -k1 -u
```

Output:
```
DEONTIC:OBLIGATORY must
IDENT              agent
IDENT              obtain_consent
TEMPORAL:U         before
TEMPORAL:U         deadline
```

Active paradigms detected: `{Deontic, Temporal}`.
Engine routing: DeonticEngine + TemporalEngine.

This is exactly what `UnicodeSemanticDictionary::detect_paradigms()` does
in Rust, but at compile-time table speed with zero process overhead.

---

## The Seven-Tier Architecture

Mapped from the previous knowledge base sessions:

```
TIER 6: EVENT INTERFACE          — External events, API gateway
TIER 5: POLICY LAYER             — HIPAA, EU AI Act, clinical protocols
TIER 4: CONTINUOUS MONITORING    — urge-monitor: obligation lifecycle, LTL
TIER 3: META-ENGINE (Fig. 26)    — urge-meta: the Figure 26 pipeline
TIER 2: ENGINE LAYER             — urge-engines: 7 paradigm implementations
TIER 1: SYMBOL ENCODING          — urge-core: Unicode Semantic Dictionary
TIER 0: LOGIC KERNEL             — urge-core: no_std AST + engine trait
```

Each tier has a well-defined interface to the tier above and below. The
`urge-monitor` crate bridges Tiers 3 and 4 — it drives the instantaneous
pipeline (Tier 3) over continuous event streams (Tier 4).

---

## The Cross-System Validation Innovation

This is the differentiating capability. Traditional policy engines evaluate
rules independently. URGE evaluates rules across paradigms and then checks
the results for inter-paradigm consistency.

### Example: Temporal-Deontic conflict

```
Obligation: O(obtain_consent, patient_P123) with deadline T+24h
Temporal:   G(consent_obtained, patient_P123)
Context:    { now: T+36h, consent_obtained: false }
```

Without cross-validation:
- Deontic engine: evaluates O(obtain_consent) → deadline not yet exceeded (wrong)
- Temporal engine: evaluates G(consent_obtained) → false
- No conflict flagged. System may proceed.

With URGE cross-validation:
- Deontic engine: O(obtain_consent) deadline=T+24h, now=T+36h → VIOLATED
- Temporal engine: G(consent_obtained) → false
- Cross-validator: temporal says false, deontic says violated → CONFLICT
- Final verdict: DENIED with conflict_detail = "temporal deadline exceeded: obligation violated"

This is what `CrossValidator::validate()` implements. One caveat: the deadline on the
deontic side needs an `Expr::DeonticStatement`, which the parser does not build yet
([#6](https://github.com/toneron2/URGE/issues/6)), so from an expression string the same
conflict arises as `must authorized and always audit_running` with the audit trail down.

---

## Obligation Lifecycle — the State Machine

```
                  ┌─────────────────────────────────────┐
                  │                                     │
   CREATE    ─► PENDING ──activate()──► ACTIVE          │
                              │                         │
                              ├──satisfy()──► SATISFIED  │  terminal
                              │                         │
                              ├──deadline_exceeded()──► VIOLATED  │  terminal
                              │                         │
                              ├──waive()──► WAIVED       │  terminal
                              │                         │
                              └──deadline_lapsed()──► EXPIRED  │  terminal
                                                               │
   ──────────────────────────────────────────────────────────── ┘
```

Transitions are **irreversible** once terminal. This matches SDL (Standard
Deontic Logic) semantics: a violated obligation cannot be "un-violated", only
a new obligation can be created.

---

## Confidence Calculation

```
confidence = min( agreed_engines / total_engines,
                  weakest engine confidence among the verdicts )
           = u8 in [0, 255] mapped to [0.0, 1.0]
```

Every node the parser produces is handled by exactly one engine, so the agreement term is
1 of 1 and the engine term is what varies:

| Engine | Confidence of its verdict |
|---|---|
| Boolean, Deontic | 255 (certain) |
| Temporal, Modal | 204 (HIGH: the future and other worlds are open) |
| Epistemic | 204 on a permit, 153 (MEDIUM) on a deny |
| Fuzzy | the membership degree, 0.5 → 127 |
| Paraconsistent | 255 when consistent, 102 (LOW) for BOTH or NEITHER |
| none succeeded | 0 |

A connective decided from its sides takes the weaker side. The pipeline then denies any
verdict under `PipelineConfig::confidence_threshold`, so under the healthcare tier (204) a
temporal clause passes exactly at the line and a fuzzy clause at 0.5 does not. If two engines
ever handle the same node, the agreement term engages as designed: a 2/3 split gives 170/255.

In healthcare config (`exhaustive_evaluation = true`) every engine is offered every node;
each node is still taken by the one engine that handles it. What the config buys is that no
paradigm is skipped by the token-based detector.

---

## Memory Model

### Embedded (no_std, no alloc)

Design targets. This tier does not compile yet; see `ROADMAP.md`.

```
Unicode dict table:    8 KB Flash (static slice)
Engine SWITCH code:    4 KB Flash
Shallow AST nodes:     2 KB Stack (depth-limited)
Obligation table:     16 KB SRAM (256 × Obligation struct)
Logic trace buffer:    8 KB SRAM (64 × TraceEntry)
─────────────────────────────────────────────────
Total:                ~38 KB
```

Compare: Python interpreter ≈ 8 MB. This is the ~1/200th the size figure.
A full Python governance system: ~200×–1000× larger depending on libraries.

### std (alloc)

Unbounded. The AST can represent arbitrarily complex nested expressions.
The logic trace grows with evaluation depth. The obligation manager handles
arbitrary numbers of active obligations.

---

## Adding a New Logic Engine

1. Add a variant to `urge_core::engine::Paradigm` enum.
2. Implement `urge_core::engine::LogicEngine` for a unit struct.
3. Register in `urge_engines::all_engines()`.
4. Add symbol entries to `UnicodeSemanticDictionary::ENTRIES`.
5. Add cross-validation rules in `urge_meta::validator::CrossValidator`.

The interface is stable — adding a new engine does not change the pipeline.

---

## Healthcare Compliance Traceability

Every verdict anchors to the audit trail through `formal_notation` and
optionally `citations`. A complete HIPAA audit response looks like:

```
VERDICT: DENIED
  Expression:     "must authorized and always audit_running"
  Formal:         "O(authorized) ∧ G(audit_running)"
  Confidence:     95%
  Paradigms:      [Boolean, Deontic, Temporal]
  Conflicts:      0
  Trace:
    [Tokenization]        tokenizing input
    [ParadigmDetection]   deontic
    [ParadigmDetection]   temporal
    [AstConstruction]     building AST
    [EngineRouting]       DeonticEngine
    [EngineRouting]       TemporalEngine
    [EngineEvaluation]    obligation active, authorized=false → DENIED
    [EngineEvaluation]    G(audit_running): currently holds
    [CrossValidation]     deontic denial overrides majority
    [VerdictSynthesis]    confidence threshold met
  Citations:
    HIPAA §164.312(a)(1) — Access Control
    HIPAA §164.312(b) — Audit Controls
```

This is the output that satisfies the question: *"Why did the system deny this action?"*

---

*Architecture document v0.1.0*
*TODOMODO.IO AGENCY LLC / Anthony R. Slosar*
