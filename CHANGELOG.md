# Changelog

## 0.1.4

Corrections:

- An identifier starting with a letter the dictionary uses as an operator's formal symbol
  (`Patient_consent`, `Flag`, `Guard`, `Obligation`) tokenized as that operator followed by
  the rest of the word, and the expression was denied with "no engines succeeded". ASCII
  letters and digits are no longer looked up as operator codepoints.
- The conflicts a conjunction reported changed with the order of its conjuncts:
  `must a and always b and must c` reported none where `must c and must a and always b`
  reported one. The cross-validator now compares every verdict of each paradigm, and an
  enclosing `and` checks the leaves of a nested conjunction. The temporal-deontic conflict
  reads "temporal constraint violated while an obligation holds"; it was "temporal deadline
  exceeded", which no unbounded `always` involves.
- A permission or prohibition that lapsed emitted a `DeadlineExceeded` violation event
  while transitioning to `Expired`. Only an unperformed obligation emits one now.
- `a xor b` printed as `(…) ∧ (b)`. It prints as `(a) ⊕ (b)`.
- The `regex` crate was a default dependency of `urge-meta` and was never used. It is
  removed; the `regex` feature name remains as an alias for `std`.
- README: the integration test count.
- Confidence came from the engines' agreement alone. Every clause is decided by one engine,
  so every verdict carried 255 and the threshold tiers never changed one. Confidence is now
  the lower of the agreement and the weakest deciding engine's own confidence: a temporal,
  modal or epistemic permit carries 204 (0.80), a fuzzy clause its degree.
- Comparisons, `release`, weak-until and deontic sides under `until` or `before` parsed and
  were then denied with "no engines succeeded" in every context. They now evaluate.
- A number in boolean position read as false. It reads as true when nonzero, as
  `Literal::as_bool` already did; a string fact still reads as false.
- `2.5` parsed as the integer 0. It parses as a float.
- `clinical::INFORMED_CONSENT` read `must consent_obtained before procedure`, which permits a
  procedure without consent once `before` is evaluated. It reads
  `procedure implies must consent_obtained`.
- The fuzzy engine read every variable as 0.5, so `a fuzzy_and b` held in every context. It
  reads a fact as its degree: a number clamped to [0, 1], a boolean as 1 or 0, an absent
  fact as 0.
- A continuous `G(φ)` monitor reported satisfied after one true observation. It stays
  undetermined while φ holds and is violated the first time φ does not.
- `PipelineConfig.depth_limit` was never read. It now caps the context's limit, so the
  `embedded` tier's limit of 8 applies.
- README and ARCHITECTURE describe what ships: 63 dictionary entries across 8 paradigm tags,
  measured latency of a few microseconds, and the embedded memory table as design targets.

Additions:

- Comparisons over facts and literals: `eq neq lt lte gt gte` and `= ≠ < ≤ > ≥`.
- `both φ` and `neither φ` reach the paraconsistent engine, and `mu φ` the fuzzy engine.
- Probabilistic keywords fail to parse with "probabilistic operators have no engine yet".
- `Policy` (`urge-runtime`, healthcare): an expression with the citation it implements.
  `HealthcareGovernor::evaluate_policy` puts the citation on the verdict and in the audit
  log with the agent and correlation id. `check_phi_access` decides `hipaa::PHI_ACCESS`
  and audits under the patient id; its agent, patient and `audit_active` arguments were
  ignored before. `AuditEntry` gains `citations` and `agent`, and `AuditLog::entries`
  returns the whole log.
- `GovernanceMonitor::watch` and `observe` drive the LTL monitors from the facts of each
  instant. `waive(id, by)` records who waived an obligation, and `forbidden_attempt(agent,
  action)` produces the event the obligation manager already accepted.

Breaking changes:

- `hipaa::*` and `clinical::*` are `Policy` values, not `&str`. Use `.expression` where a
  string is wanted.
- `Expr::Ternary`, `Expr::Quantified` and `Tokenizer.normalize_unicode` are removed. The
  parser never built the variants and nothing read the field.
- `EvalContext::get` takes `&str`.

## 0.1.3

Corrections:

- `must a or b`, `always a implies b` and every other connective after a prefix operator
  were decided as `and`. The router returned the sides' verdicts beside the connective's,
  and the validator let a denying side override it. One verdict now comes back for the
  connective, decided from its sides.
- A refusal on a conjunction of obligations reported 60-80 % confidence, an artefact of
  comparing different propositions. Confidence is now the weaker side's engine agreement.
- `never φ` read as G(φ). It now reads as G(¬φ).
- `knows`, `believes` and `common_knowledge` parsed as the literal false. They now parse as
  `knows agent φ`, `believes agent φ` and `common_knowledge φ`.
- `must_not φ` printed as F(φ), the letter `eventually` prints. It now prints as O(¬φ).
- `within`, `before` and `deadline` after a temporal operand were dropped. `eventually φ within N`
  now requires φ by logical time N and `always φ within N` requires it until then; the
  notation prints the bound (F≤N, G≤N). A bound after `next` is a parse error.
- Input that did not parse was dropped without notice, and an operand position holding an
  operator became the literal false. Both are now parse errors: the expression is denied and
  the notation names the first token that could not be placed.
- The browser build in `docs/demo/pkg/` was 0.1.1. It is rebuilt at 0.1.3, with SHA-256 sums.

Additions:

- `GovernancePipeline::explain`: each clause under the expression's connectives, the facts
  it read (a fact not supplied is reported as absent), and the clauses that decided a deny.
- The JSON call (`urge-meta`, feature `json`) adds `because` to a denied verdict, and
  `version`. The browser build and `urge-eval` share it.
- `urge-eval` (`crates/urge-cli`): one decision from the command line, JSON in and JSON out,
  exit status 0, 1 or 2.
- README: how conflicting engine results resolve, and the expression syntax.
