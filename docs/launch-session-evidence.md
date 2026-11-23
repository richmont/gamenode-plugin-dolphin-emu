# Launch session: reference-host evidence

Date: 2026-11-22. Branch: `mvp-dolphin-launch`, based on `feat/durable-catalog` at `c099f82`.
Desktop host reference: `cc667cbf162544b5446a2848c2a9decf91148dd3` (`integration/third-party-plugins`,
host-approved runtime picker). SDK path dependency: `../game-node-desktop/crates/plugin-sdk`.

This file records what the in-repo reference host proves. It does **not** claim native GameNode
acceptance: task 5.3 (real Dolphin/GameCube/Wii) and the Windows handoff remain open.

## Implemented flow

`launch/src/main.rs` opens the target-scoped extension session. A readiness startup has no
`GAMENODE_PLUGIN_USER_INTENT_ID` and exits without launching. For Play it calls
`DolphinLaunch::play` and then keeps the session open with `run_extension` while the host's launch
job rechecks its approval fence.

`DolphinLaunch::play`:

1. Reads `LibraryGet` for the host-selected installation and refuses an empty/mismatched catalog
   association.
2. Requests the installation folder with `FilesRequestAccess` (`game_id`, `installation_id`,
   `["read"]`, `["installation"]`, no declared locations), lists it and selects the ROM.
3. Reuses `RuntimesGet`/`RuntimesList` when the stored id, revision and shape are current; otherwise
   calls `RuntimesRequestAccess` with no executable handle and the `dolphin-emu` /
   `dolphin-emu.desktop` hints.
4. Reuses the stable content binding or registers it with `GamesRegisterContent` using the stored
   binding revision (0 only for creation).
5. Reuses the stored recipe or proposes one with `GamesProposeCustomLaunch`; requests execution
   with `GamesRequestCustomLaunch`, the current entry revision and the host intent unchanged.
6. Persists only runtime/content/recipe IDs and revisions plus ROM name/size/modification time in
   private `Storage` with `PERSISTENT` durability.

A stale-recipe refusal before any job is created is repaired by one fresh content registration,
proposal and request. A transport timeout or any reply that created a job is not retried, so the
extension cannot start Dolphin twice.

## Contract decisions

| Item | Value |
| --- | --- |
| Extension capabilities | `provider.context.read`, `game.launch`, `game.custom_launch`, `game.runtime.read/create`, `files.read`, `library.read`, `library.read.all`, `library.discovery`, `storage.read/write/persist`, `notifications.publish` |
| Runtime request | name `Dolphin`, version `unknown`, empty executable handle, formats `["rom"]`, platform host OS, args `-b -e {content}`, hints `dolphin-emu` / `dolphin-emu.desktop` |
| Content binding | kind `rom`, supported platform host OS, stable content id across moves |
| Recipe | library entry revision as `expected_revision`; no executable, URI, argv, cwd, environment or mod layers |
| Storage key | `launch.v1.<installation_id>`, hashed when the full key would exceed 200 bytes |
| Storage value | schema 1; installation/game IDs, ROM name/size/mtime, runtime/content/recipe ids and revisions |
| Never stored | file handles, executable handles, paths, argv, environment, credentials, intents |

`library.read.all` is required because GameNode's `LibraryGet` scope for an extension only sees its
own (empty) provider entries; the whole-library grant exposes the selected provider entry. The
extension reads only the target installation. The manifest documents this explicitly.

## Reference-host tests

`launch/tests/launch_reference_host.rs` runs a real gRPC `ResourceHost` fixture and the production
`SdkHost` adapter. It asserts the exact request sequence and wire fields:

- first Play: library read, storage read, installation access (no declared location), directory
  list, runtime list, runtime access request (empty handle, text hints), content registration
  (empty content id, revision 0, ROM handle, `rom`, host OS), interim storage write, recipe
  proposal (no overrides, intent), final storage write, execution request (recipe id/revision,
  current entry revision, intent, no override fields);
- opaque handles are passed through and never persisted; the stored JSON contains no handle name,
  intent or directory handle;
- repeated Play reuses the runtime, content binding and recipe, does not re-register content and
  does not propose again; it sends a new intent unchanged;
- a changed runtime revision goes back through `RuntimesRequestAccess`; a changed content/ROM
  (name, size or modification time) creates a new binding revision and a new recipe;
- a stale recipe refusal is repaired once with exactly one job; error paths (unmatched game,
  denied files, denied runtime, missing ROM, denied launch, no intent) create zero jobs;
- `Initialize` and `Check` return `Completed` without any resource call or job.

`launch/src/lib.rs` unit tests cover the pure builders, the exact files/runtime request shapes,
ROM selection ambiguity, durable-record privacy, storage key bounds and runtime shape matching.

## Command/result matrix (2026-11-22)

All Cargo commands used:

```text
CARGO_TARGET_DIR=/data/DADOS_LINUX/geral/area_troca_rust/target-dolphin-launch
CARGO_NET_OFFLINE=true
CARGO_INCREMENTAL=0
```

| Command | Result |
| --- | --- |
| `cargo test -p gamenode-dolphin-emu-launch` | 10 unit + 8 reference-host tests passed |
| `cargo test --workspace` | launch 10+8, provider 16+4 passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean after formatting one pre-existing provider line |
| `git diff --check` | clean |

## Gaps and host dependencies

- `LibraryEntry` exposes `game_id`, `installation_id`, name and revisions, not the provider's
  `rom`, `platform` or `title` fields. The extension selects the ROM from the approved listing by
  normalized title, with a unique-file fallback, and refuses ambiguity.
- The content approval has no digest. Size and modification time detect common in-place changes;
  a replacement preserving both cannot be distinguished without a host content-identity API.
- The SDK's `files_request_access` and `runtime_request_access` helpers use a 15-second deadline
  while the host waits up to five minutes for the user. The extension builds those two requests
  explicitly with a five-minute deadline. The host's supervised extension process also has a
  five-minute lifetime, so a slow approval can be cut off; this needs native acceptance evidence.
- GameNode does not dispatch a command to an extension action session. The work therefore runs
  from `main`; `ExtensionPlugin::handle` is readiness-only and never launches.
- The existing signed `.gnplugin` artifacts and their recorded hashes predate this change. Rebuild,
  re-sign and re-verify before native acceptance; the release tag and bundle were not touched.
- Native Linux x86_64 GameCube/Wii acceptance (5.3) and Windows handoff (1.7 in the desktop plan)
  remain open. The reference host proves the client contract, not GameNode's production launch,
  consent dialog, picker UI, supervisor or playtime path.

## Local 0.3.0 test bundle (2026-11-22)

- Built from this worktree as version 0.3.0, signed with a new development key because the 0.2.0
  key lived only in `/tmp` and was cleaned. This is a local test artifact, not a release.
- Publisher key id `richmont-dolphin-emu-development`; fingerprint
  `SHA256:88d96d09e9215555c7d99cd8abfdb51822860d63aaa5013b403dae789c0caf9a`.
- Bundle: `/home/rich/gamenode/gamenode-plugin-dolphin-emu/dist/dolphin-emu-0.3.0-linux-x64.gnbundle`;
  SHA-256 `97e3ef048c2a581a89178541bd9fc1f37719fb2927a777c4fcad084f4740f9c9`.
- The 0.2.0 publisher key cannot verify this bundle, so importing it requires a fresh trust review
  (or uninstalling 0.2.0 with Keep data first). No tag, push or GitHub release was made.

## 0.3.1 target-revision fix (2026-11-22)

- Production fences an extension launch request against the **session target revision**
  (`TargetGameContext.revision`), not the `LibraryGet` entry revision. They are different values
  on a real host (the Dolphin fixture uses target 100 and entry 7). The extension now parses
  `host.target().revision` and sends that value to `Propose`/`Request`; the reference host fixture
  was corrected to fence on the target revision, since its old entry-revision check hid the bug.
- Bundle: `/home/rich/gamenode/gamenode-plugin-dolphin-emu/dist/dolphin-emu-0.3.1-linux-x64.gnbundle`;
  SHA-256 `f6686e37538bb2ca794d8d4ef42d599371f55d92e97488f0246eace7ebc5dac9`; same development
  publisher fingerprint as 0.3.0. Local test bundle, no tag or push.

## 0.3.2 re-key and Windows target (2026-11-22)

- The 0.3.1 development private key lived under `/tmp` and was lost with a reboot. 0.3.2 is signed
  with a new development key stored at `~/.config/gamenode/plugin-keys/dolphin-emu-dev.pem` and in
  Bitwarden; the publisher `public_key` changed, so a fresh trust review is required.
- Both components now declare `linux/x86_64/gnu` and `windows/x86_64/msvc`. The Windows binaries
  are cross-built MSVC with the static CRT (no Visual C++ Redistributable); native Windows
  acceptance is not recorded.
- Bundle: `/home/rich/gamenode/gamenode-plugin-dolphin-emu-launch/dist/dolphin-emu-0.3.2-x64.gnbundle`;
  SHA-256 `bcd3e603ff7e259e961cbc1f84f96cddb1d0617a490a2396bdc3ff328f19aee4`. Local test bundle,
  no tag or push.
