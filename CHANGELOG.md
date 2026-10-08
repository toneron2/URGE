# Changelog

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
- `within`, `before` and `deadline` after a temporal operand were dropped. `always φ within N`
  now sets the bound.
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
