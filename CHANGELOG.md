# Changelog

## Unreleased

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
