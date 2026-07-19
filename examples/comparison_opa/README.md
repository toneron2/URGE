# URGE vs OPA — one policy, side by side

This folder backs the comparison table in the main README with something you
can run. One scenario, both engines, actual outputs.

**Scenario.** A payment agent wants to execute `submit_payment`. Policy:

1. the agent **must be authorized** — an obligation (deontic logic);
2. the compliance review **must be complete throughout** — a standing
   constraint (temporal logic, LTL `G`);
3. the review itself is an obligation **with a deadline** — a lifecycle,
   not a boolean.

## The OPA side

[`policy.rego`](policy.rego) — both constraints collapse to booleans:

```rego
package agent.gate

default allow := false

allow if {
	input.authorized
	input.review_completed
}
```

```
$ opa eval -d policy.rego -i input_ok.json             'data.agent.gate.allow'
→ true
$ opa eval -d policy.rego -i input_review_missing.json 'data.agent.gate.allow'
→ false
```

(Outputs verified with OPA v1.18.2; JSON trimmed to the value.)

Correct, fast, and done: allow or deny. What the `false` cannot tell you is
*what kind* of failure this was — that an active obligation stands in
contradiction with a violated temporal constraint — and Rego has no notion of
the review being an obligation with a deadline that can pass.

## The URGE side

Same policy as one expression: `must authorized and always review_completed`.
Runnable version: [`crates/urge-runtime/examples/comparison_opa.rs`](../../crates/urge-runtime/examples/comparison_opa.rs)

```
$ cargo run -p urge-runtime --example comparison_opa
```

Actual output (v0.1.1):

```
input: authorized=true, review_completed=true
→ verdict:    PERMIT
→ formal:     O(authorized) ∧ G(review_completed)
→ confidence: 100%
→ consistent: true (conflicts: 0)

input: authorized=true, review_completed=false
→ verdict:    DENY
→ formal:     O(authorized) ∧ G(review_completed)
→ confidence: 67%
→ consistent: false (conflicts: 1 — temporal deadline exceeded: obligation violated)
→ trace (22 entries), last stages:
    [CrossValidation] cross-system validation
    [CrossValidation] CONFLICT: temporal constraint violated while obligation active
    [CrossValidation] cross-validation: conflicts detected
    [VerdictSynthesis] confidence threshold not met — deny

=== Obligation lifecycle (no OPA equivalent) ===

t=0     obligation registered: complete_review, deadline t=1000
t=2000  time tick — deadline exceeded, review never happened
        VIOLATION EVENT: DeadlineExceeded { id: ObligationId("review-P123"),
        agent: "payment-agent", action: "complete_review",
        deadline_ns: 1000, detected_at: 2000 }
```

Same deny — plus the classification of the failure (a deontic-vs-temporal
contradiction, reported rather than silently resolved), the formal notation,
a full stage trace for the audit record, and the deadline tracked as a
first-class obligation that emits a violation event when it lapses.

## What OPA does better

Credibility requires symmetry, so plainly: OPA is a mature, CNCF-graduated
project with years of production hardening, a policy language (Rego) built for
querying deeply nested JSON documents, a huge ecosystem (Kubernetes admission
control, Envoy, Terraform), decision logging, bundle distribution, IDE
tooling, and a test framework. URGE is a v0.1 library with none of that
infrastructure. If your policies are single-logic questions over structured
documents — "may this pod be admitted?" — OPA is the right tool and this
comparison changes nothing.

URGE's contribution is narrower and different: evaluating one governance
expression under several formal logics at once, cross-checking the results
between paradigms, and tracking obligations over time. Use it where the
*reasoning* about an action needs to be formal, multi-paradigm, and auditable
— e.g. as the governance gate in front of an autonomous agent.
