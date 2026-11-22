# Tasks

Implementation is pending. Check an item only when its evidence exists; docs and the fixture run
are part of the deliverable, not follow-ups.

## 1. Discovery

- [x] 1.1 Add `dol` and `elf` to `provider/src/roms.rs` `ROM_EXTENSIONS` and keep the existing
  case-insensitive predicate; verify a mixed-case `.DOL`/`.ELF` is accepted and `notes.dol.txt` is
  not.
- [x] 1.2 Skip the filename disc-ID heuristic for `dol`/`elf` and fall back to the existing
  `dolphin-emu-<hash>` identity; verify `GZLE01 demo.dol` reports no `game_id` and keeps the opaque
  ID.
- [x] 1.3 Keep `platform` `unknown` for headerless executables and assert the record still carries
  the existing `rom`, `platform` and `game_id` fields so the launch path is unchanged.

## 2. Durable catalog

- [x] 2.1 Emit `disc_id` only when a disc ID exists and omit the field otherwise, leaving dataset,
  schema version, durability, observation fields and `resource_id`/`content_id` unchanged.
- [x] 2.2 Extend the catalog tests: headerless payload has no `disc_id` key, a disc payload still
  has it, and no path, credential or session value appears in either payload.

## 3. Tests and documentation

- [x] 3.1 Add provider tests for DOL offsets, ELF magic, extension case, opaque identity and the
  rename rule, keeping the existing scan regressions green.
- [x] 3.2 Synchronize `README.md`, `TASKS.md`, `docs/architecture.md` and `docs/protocol-notes.md`
  with homebrew support, the `unknown` platform, the conditional `disc_id` and the rename caveat.

## 4. Acceptance

- [ ] 4.1 Run the 240p Test Suite `.dol` fixture (kept outside Git with version and SHA-256)
  through the host: discovery, library entry, approval, launch, exit detection, playtime and repeat
  Play, recording host/plugin revisions and the outcome of the `unknown`-platform binding.
- [ ] 4.2 Record the evidence in `docs/` (fixture version and hash, screenshot, host result) and
  check off the tasks above that the evidence supports.

## 5. Validation

- [x] 5.1 `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `git diff --check` and
  `openspec validate --all --strict --no-interactive` with the external Cargo target, all passing.
