# Stale examples (quarantined)

These two examples were written against an **earlier URGE API** (pre-February 2026
rewrite) and no longer compile. They import a top-level `urge` facade crate that
does not exist in the current workspace, plus types that were renamed or removed:

| Old symbol (in these files) | Current equivalent |
|-----------------------------|--------------------|
| `urge` (umbrella crate)     | `urge-runtime` (re-exports the surface) |
| `GovernanceEngine`          | `GovernancePipeline` (`urge-meta`) |
| `Context`                   | `EvalContext` (`urge-core::engine`) |
| `Request`, `ActionKind`, `WorldState`, `PolicySuite`, `ast::{Agent, Atom}` | removed / redesigned |

They are kept here for reference only. Working, compiling examples live in
[`crates/urge-runtime/examples/`](../../crates/urge-runtime/examples/):
`agent_gate.rs`, `bios_access.rs`, `healthcare_consent.rs`.

To revive one of these, port it to the current API and move it back under
`crates/urge-runtime/examples/`.
