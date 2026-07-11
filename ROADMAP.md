# URGE — Status & Roadmap

_Last updated: 2026-07-11_

## Current status: v0.1.0 (initial public release)

The **`std` / `alloc` tiers are complete and tested.** URGE compiles clean,
passes its full test suite, and is lint- and format-clean.

| Area | State |
|------|-------|
| Workspace (`urge-core`, `-engines`, `-meta`, `-monitor`, `-runtime`) | ✅ builds clean |
| Tests | ✅ 35 passing (unit + doctests + 19 end-to-end integration) |
| `clippy -D warnings` | ✅ clean (default **and** `--all-features`) |
| `rustfmt` | ✅ clean |
| `serde` feature | ✅ functional (Serialize for zero-alloc trace/verdict types) |
| Examples | ✅ `agent_gate`, `bios_access`, `healthcare_consent` |
| CI | ✅ GitHub Actions (test / clippy / fmt / docs) |
| Embedded `no_std` + no-`alloc` tier | ⚠️ **experimental / does not compile yet** |

## Known limitation — the embedded (heap-free) tier

The `no_std` + no-`alloc` build **does not compile**. The AST is recursive
(`Expr` contains `AstNode`, `AstNode` wraps `Expr`), and the no-alloc definition
`pub struct AstNode(pub Expr)` has no indirection → an infinitely-sized type.

The `alloc` tier works because `AstNode = Box<Expr>` provides the indirection.
The heap-free tier needs a **heap-free representation** — most likely an
**arena + index** design (nodes stored in a fixed-capacity `heapless::Vec`,
`AstNode` becomes a `u16` index). This touches the AST, all engines
(`node.as_ref()`), and the parser, so it is a dedicated piece of work.

Until then, the embedded story is marked experimental in the README, the
`urge-runtime::embedded` module, and CI (`test-no-alloc` is non-blocking).

## Next steps (roughly in priority order)

1. **Heap-free AST** — arena/index representation so the `no_std` tier compiles
   and the embedded/BIOS claims become real. (Biggest item.)
2. **Publish to crates.io** — free, and the main discovery channel for Rust
   crates (auto-builds docs on docs.rs). Requires claiming the `urge-*` names
   and a crates.io token. _Irreversible name claim — do intentionally._
3. **Benchmarks** — the `benches/` dir is empty; add Criterion benches to back
   the latency claims in the README.
4. **More worked examples** — port the two quarantined pre-Feb examples
   (`docs/stale-examples/`) to the current API, or replace with fresh ones.
5. **API docs pass** — ensure every public item has doc examples that compile.

## How it got here

The Feb 2026 rewrite left the workspace non-compiling. The 2026-07-11 session
fixed: a workspace-manifest bug, five compile errors, a completely broken
`serde` feature, a real self-assignment bug, and wired the integration tests
(which had been orphaned in the virtual-workspace root and never ran). See the
initial-release commit for the full list.
