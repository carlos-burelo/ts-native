# Contributing to TSN

## Development setup

1. Install stable Rust.
2. Clone the repository.
3. Run:

```sh
cargo check --workspace
```

## Mandatory checks before opening a PR

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace
cargo run --release --bin tsn -- ./examples/production-test.tsn
```

## Change policy

- Keep changes scoped to one concern.
- Preserve language semantics unless the PR explicitly introduces a breaking change.
- Add or update tests/examples when behavior changes.
- Do not commit build artifacts (`target/`, logs, binaries).

## Commit guidelines

- Use clear messages in imperative mood.
- Reference issue IDs when applicable.
- Include a short risk/impact note in PR description.
