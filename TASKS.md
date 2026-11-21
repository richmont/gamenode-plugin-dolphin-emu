# Tasks

Ordered work for the next contributor. Each task states the acceptance evidence; keep it
unchecked until that evidence exists.

The MVP's detailed execution checklist is in
[the active OpenSpec change](openspec/changes/complete-dolphin-gamenode-integration/tasks.md).
This file retains the broader roadmap; synchronize both when recording implementation evidence.

## 0. Identity, metadata and persistence prerequisites

- [ ] Separate `disc_id` (and GameTDB ID where different), platform, revision and disc number
  from GameNode catalog `game_id`, installation ID and content ID. Read legacy `fields.game_id`
  as disc metadata; define copy/multidisc identity and migration before changing deduplication.
- [ ] Resolve titles from Dolphin's local `Load/wiitdb.txt` or `Load/titles.txt`, then
  `Sys/wiitdb-<language>.txt` with English fallback, ROM metadata and file name. Detect data
  directories separately from configuration roots and bound file size/record count. Full synopses
  are outside the MVP; no binary GameListCache dependency is required.
- [ ] Read existing library matches; search/assign catalog candidates through the Matching API.
  Preserve user choices and expose ambiguous/unmatched cases instead of inventing a catalog ID.
- [ ] Use host Storage endpoints with persistent durability for versioned, bounded metadata and
  auxiliary matching records. Add the needed library/matching/storage permissions to the provider.
  Implement revision-conflict, quota, denied-grant and deleted-cache behavior.
- [ ] Store launch references in the extension's own namespace with the required storage grants;
  exchange game state through the library API, never through the provider's private keys.

Acceptance: fixtures cover local title precedence, missing/malformed databases, language fallback,
distinct copies/discs, ambiguous matches, preservation of user decisions, persistent restart,
schema migration, account/component isolation and storage failures. A cache entry alone never
counts as a host catalog match.

## 1. Finish the launch session (MVP critical path, after prerequisites)

Implement `launch/src/lib.rs` `ExtensionPlugin::handle` for the target-game session:

1. Resolve the selected `TargetGameContext` (`host.context().target_game`).
2. Read the library entry with `host.resources()?.library_get(installation_id)` and take `rom`
   and `platform` from its fields. Use the host-selected/catalog game ID for RPCs, never the disc
   ID from `fields.game_id`. Reject an unresolved catalog association with actionable guidance.
3. Ask for the ROM handle with `FilesRequestAccess` (`game_id`, `installation_id`,
   `resource_kinds = ["installation"]`, no declared locations). The reply is a directory/file
   handle bound to this session; list the installation folder and pick the file named by `rom`.
4. Resolve the owned approved runtime by durable identity; a display name or shared catalog row
   alone is insufficient. If absent, register from an approved executable handle (see below).
5. Add `library.discovery` to extension capabilities and manifest alongside `files.read` for
   content registration. Reuse a stable content ID and its binding revision; use revision 0 only
   for creation. Keep the library revision separate from binding and recipe revisions.
6. Propose the recipe once (`propose_custom_launch`) and request the approved revision on later
   plays (`request_custom_launch`). Use `Launch::user_intent_id()` unchanged; never invent or
   reuse an intent.
7. Map refusals to actionable text: no intent, denied grant, stale revision, missing runtime,
   missing content, incompatible package.
8. Persist durable runtime/content/recipe IDs and revisions through host Storage, then revalidate
   on later sessions. Never persist session handles or intents. Verify first-use proposal/review/
   execution semantics and ensure Initialize/Check/readiness cannot cause duplicate launches.

Acceptance:

- fixture test over the SDK reference host asserting the exact call sequence, the opaque handles
  and that no argv/executable/environment override is sent;
- manual Linux x86_64 runs in GameNode for GameCube and Wii: approve the runtime, start with the
  extension, select/reuse the recipe for later Play, close Dolphin and verify session/playtime;
- restart/repeated-Play fixtures reuse valid references; changed ROM/runtime, expired intent,
  stale revisions and revoked grants refuse or require fresh approval without fallback;
- no process is started by the plugin in any error path.

Open question: how does the user approve the Dolphin executable on each platform? Today
`FilesRequestAccess` approves installation/save/prefix folders for a game, not arbitrary
executables. Decide and document one of:

- a host-side runtime approval UI (preferred; runtime registration already needs a host-approved
  handle);
- a declared `prefix` folder containing Dolphin, approved by the user, with the executable handle
  taken from it;
- a Core-provided runtime the extension only *uses*, conditional on a verified ownership and
  execution-authority contract (shared catalog visibility alone is not enough).

Do not work around this by spawning Dolphin from the plugin.

## 2. Provider robustness

- Parse RVZ/WBFS/WIA headers for the real disc id and title; currently they fall back to the file
  name. Add fixtures with real headers (small synthetic files are enough).
- `tokio::task::spawn_blocking` already runs the scan; retain it and verify responsiveness.
- Windows paths: `Dolphin.ini` may quote paths or use forward slashes; normalize and test.
- Multiple Dolphin profiles / Flatpak and native configs; deduplicate roots; ignore missing roots
  silently.
- Correct identity-based deduplication independently of display-name sorting; cover overlapping
  roots, copied ROMs, multiple revisions and multidisc titles.

Acceptance: unit tests for every parser, a scan with >10k entries, and explicit depth/50k-entry
boundary, symlink and overlapping-root tests. Existing tests do not establish those limits.

## 3. Covers and metadata

- Fetch covers from GameTDB (`https://art.gametdb.com/wii/cover/...` and GameCube equivalent) with
  `network.fetch`, exact approved origins, bounded responses, a quota-aware host-managed cache
  and a clear privacy note.
- Set them with `ArtworkSet` or the cover helpers; never fetch during discovery without the grant.
- Title resolution belongs to prerequisite 0. Rich descriptions are optional enrichment after
  matching; the local Dolphin title database is not a complete synopsis catalog.

Acceptance: fixture test for the URL shape, cache hit/miss, denied grant and offline behavior.

## 4. Saves

- Declare Dolphin save profiles per game: GameCube memory cards (`User/GC/<region>/Card A`) and
  Wii title data (`User/Wii/title/<id>/data`), using `Saves` profiles with approved folders.
- Do not parse or convert saves.
- Resolve actual configured save locations and shared memory-card ownership; validate host
  backup/restore coordination against running games before promising per-game isolation.
- Acceptance: GameNode lists the profile, backup/restore works, a moved/removed folder reports
  missing without deleting history.

## 5. Packaging and release

- Prepare two signed Linux x86_64 development packages for the MVP acceptance in task 1; this
  prerequisite does not wait for bundle support or later feature work.
- Ship both components as one signed bundle once GameNode implements
  `openspec/changes/add-plugin-bundles`: one `.gnbundle` with the two existing `.gnplugin`
  payloads, one publisher pin, one review, one inventory row and one uninstall. Until then the
  README's two-package flow is the only installable path.
- Replace the placeholder publisher keys with the release key; keep development keys out of the
  repository.
- Generate the real third-party notices from the locked graph (`cargo metadata`) and review them.
- Add CI: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, package verification
  with the GameNode pack tool.
- Document the supported API range and the SDK version policy.
- Linux x86_64 is the MVP target, not a permanent limit. Add Windows releases and consider macOS
  and other architectures. Each shipped OS/architecture/ABI needs a binary and a corresponding
  `targets` entry in both manifests; never advertise unbuilt targets. This is independent of bundles.

Acceptance: both `.gnplugin` archives import in GameNode, pin the publisher and install.

## 6. Platform acceptance

- Linux native x86_64 (MVP): provider scan + metadata/matching + approved launch and playtime.
- Windows (planned release): named-pipe transport, `%APPDATA%` config path, `-b -e` arguments and process-exit
  detection. Record evidence separately from cross-builds.
- macOS (possible follow-up): config/data paths and native process supervision.
- Other architectures: build/package verification and native acceptance for each declared target.
- Flatpak: verify data discovery and the approved launch route separately from Linux native.

## 7. Documentation polish

- Keep `docs/protocol-notes.md` in step with the served API.
- Add a troubleshooting section (Dolphin path, Flatpak, permissions denied, stale recipe).
- Link the GameNode release notes for the API minor this plugin targets.
