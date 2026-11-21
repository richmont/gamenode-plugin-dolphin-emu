# Contributing

## Ground rules

- This is a third-party plugin: depend only on the public `gamenode-plugin-sdk` (plus ordinary
  crates such as `tokio` and `async-trait`). Never import GameNode Core, platform or application
  crates, and never copy their internal types.
- The plugin process is not sandboxed. Treat every path, handle and credential as sensitive and
  never log secrets.
- Do not start, signal or reap the emulator from the plugin. The host owns the process tree.
- Never send a path, argv, working directory or environment override in an approved recipe
  request.

## Commits

Use Conventional Commits in English, imperative, with a lowercase type and a scope:

```text
feat(launch): request the approved Dolphin recipe
fix(provider): read quoted ISOPath values
docs(protocol): describe the runtime approval
```

Keep unrelated changes in separate commits. Do not rewrite published history.

## Validation

Use an external target, never a checkout-local `target/`:

```sh
CARGO_TARGET_DIR=/data/DADOS_LINUX/geral/area_troca_rust/target-dolphin-plugin cargo fmt --all -- --check
CARGO_TARGET_DIR=/data/DADOS_LINUX/geral/area_troca_rust/target-dolphin-plugin cargo clippy --workspace --all-targets -- -D warnings
CARGO_TARGET_DIR=/data/DADOS_LINUX/geral/area_troca_rust/target-dolphin-plugin cargo test --workspace
```

On other machines substitute a directory on a large, persistent disk. Before committing, also run
`git diff --check`.

## Tests

- Unit tests live beside the code (`#[cfg(test)]`). Use `tempfile` for filesystem fixtures; never
  depend on a real Dolphin installation or a real ROM.
- Integration tests over the SDK reference host belong in `launch/tests/` once the session flow
  exists. They must not execute Dolphin.
- Keep fixtures small: a six-byte disc header is enough to identify a game.

## Documentation

Update `README.md`, `docs/architecture.md`, `docs/protocol-notes.md` and `TASKS.md` in the same
change when behavior, permissions or the flow change. A task is complete only with the evidence
listed in `TASKS.md`.
