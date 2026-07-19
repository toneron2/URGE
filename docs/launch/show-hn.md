# Show HN post — DRAFT for Tony's review

> **Do not post until the live demo URL works and you've read every word.**
> Submit the demo URL as the link (HN convention for Show HN with a live
> demo), with the text below as the first comment — or submit as text post
> with both links. Best window: weekday mornings US Eastern.

---

**Title:**

Show HN: URGE – a deterministic governance gate for AI agents (Rust)

**URL:** https://toneron2.github.io/URGE/demo/

**Text (or first comment if submitting the URL directly):**

LLM agents can act, but they can't prove why an action was allowed. URGE is
the other half: the LLM proposes, a symbolic engine verifies. Wrap an agent
action in a formal policy check — `must authorized and always audit_running`
— and get back a deterministic verdict with formal notation
(`O(authorized) ∧ G(audit_running)`), a confidence score, and a
stage-by-stage reasoning trace you can store as the audit record. No model
in the loop: same input, same verdict, every time.

The distinctive part is cross-paradigm validation: the expression is
evaluated by separate formal engines (deontic obligations, temporal LTL,
epistemic, fuzzy, Belnap 4-valued paraconsistent), and the results are
checked *against each other*. When the temporal deadline says "violated"
while the deontic engine says "obligation active", that contradiction is
reported and traced instead of being silently collapsed into `false`.

Demo is pure client-side WASM (~190 KB). Repo:
https://github.com/toneron2/URGE — Rust workspace, Apache-2.0, v0.1.1;
honest limitations in the README (the embedded no_std tier doesn't compile
yet, and there's a runnable OPA comparison in the repo that's fair about
what OPA does better).

---

**Prepared first comment (anticipating questions):**

A few things people will reasonably ask:

- **vs OPA:** OPA is mature infrastructure for single-logic policy over JSON
  documents; use it for Kubernetes admission. URGE's contribution is
  evaluating one expression under several formal logics and reporting
  contradictions between them, plus obligation lifecycle (deadlines that
  expire and emit violation events). Side-by-side with actual outputs:
  https://github.com/toneron2/URGE/tree/main/examples/comparison_opa
- **Why formal logic for agent governance:** behavioral alignment is
  implicit and non-auditable; a governance gate needs the opposite
  properties. Deontic logic ("must/may/must-not") + LTL ("always/until") is
  the vocabulary regulations are already written in.
- **State of no_std / embedded:** designed for it, doesn't compile yet
  (recursive AST needs an arena representation). Top roadmap item, says so
  prominently in the README.
