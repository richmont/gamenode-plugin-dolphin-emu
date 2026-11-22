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

Implemented in `launch/src/lib.rs` and `launch/src/main.rs`; reference-host evidence is recorded
in [docs/launch-session-evidence.md](docs/launch-session-evidence.md). The reference host proves
the client contract only. Native Linux acceptance and the Windows handoff remain open.

- [x] Resolve `host.context().target_game`, read `LibraryGet`, and reject an unresolved catalog
  association with actionable guidance. The entry exposes no `rom`/`platform`/`title` fields; the
  ROM is selected from the approved installation listing by title, with a unique-file fallback and
  ambiguity refusal.
- [x] Request the installation folder with `FilesRequestAccess` (`game_id`, `installation_id`,
  `["read"]`, `["installation"]`, no declared locations), list it and pass only the selected file
  handle to content registration. No path is exposed to the plugin.
- [x] Reuse `RuntimesGet`/`RuntimesList` when the durable id, revision and shape are current;
  otherwise call `RuntimesRequestAccess` with no executable handle and `dolphin-emu` /
  `dolphin-emu.desktop` hints. The host picker owns the executable; the reply has no path/handle.
- [x] Register/reuse a stable content binding with `GamesRegisterContent`; library entry, content
  binding and recipe revisions stay separate and revision 0 is used only for creation.
- [x] Propose the recipe once and request the approved revision on later plays; pass
  `Launch::user_intent_id()` unchanged. Refusals map to actionable text for no intent, denied
  grants, stale revisions, missing runtime/content and changed executable/content.
- [x] Persist only durable runtime/content/recipe IDs and revisions (plus ROM name/size/mtime) in
  private `Storage`; revalidate on later sessions. No handles, paths, argv, environment or intents
  are stored.
- [x] Manifest: added `game.launch`, `library.discovery`, `library.read.all`, storage and
  notification grants with review/impact text; removed unused runtime update/delete declarations.
- [x] Reference-host fixture: exact call sequence, opaque handles, no argv/executable/environment
  override, repeated-Play reuse, reapproval on changed runtime/content, no job on error paths and
  readiness-only handlers.

Acceptance still open:

- native Linux x86_64 runs in GameNode for GameCube and Wii: approve the runtime, start with the
  extension, select/reuse the recipe for later Play, close Dolphin and verify session/playtime
  (5.3);
- signed-package import/review and the Windows native handoff; the existing artifacts predate this
  change and must be rebuilt/re-signed without reusing the recorded hashes.

Resolved open question: Dolphin is approved through the host-side runtime picker
(`RuntimesRequestAccess`, desktop change `add-approved-runtime-picker`). There is no
prefix-folder workaround and no plugin process spawn.

Known host API gaps (documented, not worked around):

- `LibraryEntry` does not expose the provider's `rom`, `platform` or `title`;
- the content approval has no digest, so a same-size/same-mtime replacement is not detectable;
- the SDK `files_request_access` / `runtime_request_access` helpers use a 15-second deadline while
  the host picker waits up to five minutes; the extension builds those two requests with a
  five-minute deadline;
- GameNode does not dispatch a command to an extension action session, so the session runs from
  `main`; `Initialize`/`Check` remain readiness-only.

## 2. Provider robustness

- [x] Parse RVZ headers for the real disc id and internal title, and read the title from plain
  `.iso`/`.gcm` headers. Unit fixtures cover both; an env-gated `DOLPHIN_RVZ_PATH` test reads a
  real file. WBFS/WIA still fall back to the file name and need their own header offsets.
- [ ] Add WBFS/WIA header fixtures once their offsets are verified against real images.
- `tokio::task::spawn_blocking` already runs the scan; retain it and verify responsiveness.
- Windows paths: `Dolphin.ini` may quote paths or use forward slashes; normalize and test.
- Multiple Dolphin profiles / Flatpak and native configs; deduplicate roots; ignore missing roots
  silently.
- Correct identity-based deduplication independently of display-name sorting; cover overlapping
  roots, copied ROMs, multiple revisions and multidisc titles.
- [x] Report `.dol`/`.elf` homebrew executables with the path-derived id, `unknown` platform and no
  invented disc id, and omit `disc_id` from their catalog payload. Unit-tested; native host
  acceptance remains in section 6.

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
- [ ] Verify the signed bundle in the native GameNode import/review/inventory/uninstall flow;
  host bundle support is now available. Local artifact verification is recorded separately below.
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

## Durable catalog and bundle delivery (2026-11-21)

- [x] Publish the minimal durable catalog through SDK data_publish; verified granted/denied calls,
  replacement, private-field exclusion and reference-host restart while preserving discovery.
- [x] Build/sign/verify Linux components and their two-component bundle; recorded hashes and commands.
- [ ] Integration handoff: actual host consumer reads while provider is stopped; Linux bundle
  import/review/inventory/uninstall and retention; Windows cross-build and separate native checks.

The first two items are local delivery evidence; they do not close main-repo 6.3/7.1/7.2.
See [evidence](docs/durable-catalog-evidence.md). Launch blockers remain out of scope.
