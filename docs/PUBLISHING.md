# Publishing URGE to crates.io

All five `urge-*` names were confirmed available on crates.io on 2026-07-19.
Metadata (license, keywords, categories, readme, repository) is complete and
`cargo publish --dry-run -p urge-core` verifies clean.

## One-time setup (requires your credentials — do this yourself)

1. Log in at <https://crates.io> with GitHub (toneron2).
2. Create an API token: Account Settings → API Tokens → New Token
   (scope: `publish-new` + `publish-update`).
3. On this machine:

```
cargo login
# paste the token when prompted — it is stored in ~/.cargo/credentials.toml
```

## Publish (dependency order matters)

Run from the workspace root. Each step must complete before the next —
crates.io needs a minute to index a new crate before dependents can verify.

```
cargo publish -p urge-core
cargo publish -p urge-engines
cargo publish -p urge-meta
cargo publish -p urge-monitor
cargo publish -p urge-runtime
```

If a step fails with "no matching package" on a just-published dependency,
wait ~60 seconds and retry that step.

**Name claims are permanent** — crates.io does not allow deleting crates.
Versions can be yanked but names cannot be freed.

## After publication

docs.rs builds automatically within a few minutes of each publish:
<https://docs.rs/urge-meta> etc.

Add these badges to the top of `README.md` (below the CI badge):

```markdown
[![crates.io](https://img.shields.io/crates/v/urge-meta.svg)](https://crates.io/crates/urge-meta)
[![docs.rs](https://img.shields.io/docsrs/urge-meta)](https://docs.rs/urge-meta)
```

And replace the git-dependency snippet in the README Quick Start with:

```toml
[dependencies]
urge-meta = "0.1"
urge-core = "0.1"
```
