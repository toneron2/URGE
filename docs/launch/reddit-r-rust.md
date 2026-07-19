# r/rust launch post — DRAFT for Tony's review

> **Do not post until the live demo URL works and you've read every word.**
> Post as text post. Flair: "🛠️ project". Best window: Tue–Thu, 14:00–17:00 UTC.

---

**Title:**

URGE: a multi-paradigm logic/policy engine in Rust (deontic + LTL + Belnap 4-valued, with cross-paradigm contradiction detection)

**Body:**

I've been building URGE, a governance/policy engine that evaluates one
expression under several formal logics at once and cross-checks the results
between them.

Live WASM demo (no server, evaluation runs in your browser):
https://toneron2.github.io/URGE/demo/

Repo: https://github.com/toneron2/URGE

What it does, concretely: `must authorized and always audit_running` parses
into an AST whose fragments route to a deontic engine (SDL obligations) and a
temporal engine (LTL). Each returns a verdict with confidence and a trace;
a cross-validation stage then compares results *between* paradigms. If the
temporal constraint is violated while the deontic obligation is active, you
don't just get `false` — you get a reported contradiction with the reasoning
chain, plus formal notation output (`O(authorized) ∧ G(audit_running)`).

Seven engines are implemented and tested: boolean, deontic (O/P/F), temporal
LTL (G/F/X/U), modal S5, epistemic (K/B/C), Zadeh fuzzy, and Belnap 4-valued
paraconsistent (contradiction without explosion). There's also an obligation
monitor that tracks deadlines across time and emits violation events — the
part request/response policy engines don't model at all.

Honest status, because that matters more than the pitch:

- v0.1.1, Apache-2.0, `std`/`alloc` tiers complete, 43 tests, clippy/fmt clean.
- The `no_std` + no-`alloc` tier **does not compile yet** — the recursive AST
  needs an arena/index representation first. It's the top roadmap item.
- No dedicated probabilistic engine yet (operators classify, nothing evaluates).
- It is not an OPA replacement — there's a runnable side-by-side comparison in
  the repo (`examples/comparison_opa/`) that states plainly what OPA does
  better (which is: most infrastructure things).

The odd origin story: the architecture was first proved as a bash pipeline —
paradigm classification is deterministic from the token stream, so `grep`
could route "must" to a deontic bucket before any Rust existed. The README
keeps the shell proof because it's still the clearest explanation of the
core idea.

Would genuinely value: API-design feedback on the pipeline types, prior art
I should know about (I know Catala, dr_checker-style stuff, and the modal
logic crates), and anyone who wants to take a swing at the heap-free AST.

---

**Prepared first comment (post immediately after, anticipates the obvious questions):**

Anticipating three questions:

**"How is this different from OPA/Rego?"** OPA evaluates rules within one
logic, brilliantly, with years of production tooling. URGE evaluates one
expression under several logics and reports *disagreements between them*
instead of collapsing everything to booleans first. Runnable side-by-side:
https://github.com/toneron2/URGE/tree/main/examples/comparison_opa

**"Why multi-paradigm at all?"** Because governance statements genuinely mix
logics: "must obtain consent within 24h" is a deontic obligation with a
temporal deadline. Collapse it to a boolean and you lose the distinction
between "not yet satisfied", "violated", and "contradicts another rule" —
which is exactly the information an audit needs.

**"What's the state of no_std?"** Architected, doesn't compile, says so in
the README. The AST is recursive (`Box`-based on alloc); the heap-free tier
needs an arena + index representation (`heapless::Vec` + u16 handles). If
that's your kind of fun, CONTRIBUTING.md points at it.
