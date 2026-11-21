# Tasks

Ordered work for the next contributor. Each task states the acceptance evidence; keep it
unchecked until that evidence exists.

## 1. Finish the launch session (highest priority)

Implement `launch/src/lib.rs` `ExtensionPlugin::handle` for the target-game session:

1. Resolve the selected `TargetGameContext` (`host.context().target_game`).
2. Read the library entry with `host.resources()?.library_get(installation_id)` and take `rom`,
   `platform` and `game_id` from its fields.
3. Ask for the ROM handle with `FilesRequestAccess` (`game_id`, `installation_id`,
   `resource_kinds = ["installation"]`, no declared locations). The reply is a directory/file
   handle bound to this session; list the installation folder and pick the file named by `rom`.
4. Find the Dolphin runtime in `runtimes_page` (name `Dolphin`, this plugin or host-approved). If
   absent, register it from an approved executable handle (see the open question below).
5. `register_runtime_content(content_binding(...), 0, key)` and keep the returned `content_id` and
   revision.
6. Propose the recipe once (`propose_custom_launch`) and request the approved revision on later
   plays (`request_custom_launch`). Use `Launch::user_intent_id()` unchanged; never invent or
   reuse an intent.
7. Map refusals to actionable text: no intent, denied grant, stale revision, missing runtime,
   missing content, incompatible package.

Acceptance:

- fixture test over the SDK reference host asserting the exact call sequence, the opaque handles
  and that no argv/executable/environment override is sent;
- a manual run in GameNode: approve the runtime, Play a GameCube game, Dolphin opens, close it and
  the launch job ends with the session and playtime recorded;
- no process is started by the plugin in any error path.

Open question: how does the user approve the Dolphin executable on each platform? Today
`FilesRequestAccess` approves installation/save/prefix folders for a game, not arbitrary
executables. Decide and document one of:

- a host-side runtime approval UI (preferred; runtime registration already needs a host-approved
  handle);
- a declared `prefix` folder containing Dolphin, approved by the user, with the executable handle
  taken from it;
- a Core-provided runtime the extension only *uses* (no registration by the plugin).

Do not work around this by spawning Dolphin from the plugin.

## 2. Provider robustness

- Parse RVZ/WBFS/WIA headers for the real disc id and title; currently they fall back to the file
  name. Add fixtures with real headers (small synthetic files are enough).
- Run the scan on `tokio::task::spawn_blocking` so a large collection never blocks the provider
  stream.
- Windows paths: `Dolphin.ini` may quote paths or use forward slashes; normalize and test.
- Multiple Dolphin profiles / Flatpak and native configs; deduplicate roots; ignore missing roots
  silently.
- Optional: read Dolphin's `GameListCache` for titles (document the format and bounds).

Acceptance: unit tests for every parser and a bounded scan test with >10k entries.

## 3. Covers and metadata

- Fetch covers from GameTDB (`https://art.gametdb.com/wii/cover/...` and GameCube equivalent) with
  `network.fetch`, exact approved origins, bounded responses, an on-disk cache in plugin storage
  and a clear privacy note.
- Set them with `ArtworkSet` or the cover helpers; never fetch during discovery without the grant.
- Map titles/regions from the disc header if useful.

Acceptance: fixture test for the URL shape, cache hit/miss, denied grant and offline behavior.

## 4. Saves

- Declare Dolphin save profiles per game: GameCube memory cards (`User/GC/<region>/Card A`) and
  Wii title data (`User/Wii/title/<id>/data`), using `Saves` profiles with approved folders.
- Do not parse or convert saves.
- Acceptance: GameNode lists the profile, backup/restore works, a moved/removed folder reports
  missing without deleting history.

## 5. Packaging and release

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

Acceptance: both `.gnplugin` archives import in GameNode, pin the publisher and install.

## 6. Platform acceptance

- Linux native: provider scan + launch.
- Windows: named-pipe transport, `%APPDATA%` config path, `-b -e` arguments and process-exit
  detection. Record evidence separately from cross-builds.
- macOS: config path and process supervision.

## 7. Documentation polish

- Keep `docs/protocol-notes.md` in step with the served API.
- Add a troubleshooting section (Dolphin path, Flatpak, permissions denied, stale recipe).
- Link the GameNode release notes for the API minor this plugin targets.
