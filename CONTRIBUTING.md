# Contributing to URGE

Thanks for your interest — contributions are welcome.

## Quick checks before a PR

```
cargo test --all-features          # 35 tests must pass
cargo clippy --all-features -- -D warnings
cargo fmt --all -- --check
```

CI runs exactly these on every push and PR.

## Where help is most valuable

1. **Heap-free AST** (the top roadmap item) — an arena/index representation so
   the `no_std` + no-`alloc` tier compiles. See `ROADMAP.md` for the design
   sketch.
2. **Probabilistic engine** — the dictionary already classifies probabilistic
   operators; a dedicated engine is unimplemented.
3. **Examples and docs** — porting `docs/stale-examples/` to the current API,
   doctests on public items.
4. **Benchmarks** — `benches/` is empty; Criterion benches to back the latency
   claims.

## Ground rules

- Keep the WIP/experimental disclaimers honest. If something doesn't work yet,
  say so in the docs — never claim it does.
- Engine semantics changes need a test demonstrating the new behavior.
- One logical change per PR.

## Questions

Open a [Discussion](https://github.com/toneron2/URGE/discussions) or email
<anthonyslosar@gmail.com>.

## License

By contributing, you agree that your contributions are licensed under the
Apache License 2.0, the same license as the project (inbound = outbound).
No CLA.
