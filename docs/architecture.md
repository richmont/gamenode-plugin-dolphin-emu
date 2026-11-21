# Architecture

## Why two packages

A GameNode package is either a **provider** or an **extension**; it cannot be both. The split here
is not cosmetic, it matches two different host capabilities:

| Package | Kind | Job | Grant |
| --- | --- | --- | --- |
| `gamenode-dolphin-provider` | provider | Scan Dolphin's game folders and report GameCube/Wii games | `library.discovery` |
| `gamenode-dolphin-launch` | extension | Register Dolphin as a runtime and run an approved launch recipe | `game.custom_launch` + runtime/file grants |

A provider cannot launch a shared emulator through `DeclareActions`: that path is URI-only and has
no exit detection. A detected executable target (API 1.6) must live inside the game's own install
folder, which a central Dolphin installation does not. A custom launch is therefore the only path
that starts an emulator, gets the user's consent for the exact executable and arguments, and lets
the host supervise the process tree.

## Discovery

```text
Dolphin.ini (ISOPath0..N)  ─┐
DOLPHIN_PLUGIN_ROOTS       ─┴─► scan (bounded) ─► DiscoveryRecord per ROM
                                                   id: game id or stable hash
                                                   install_dir: parent folder
                                                   fields: rom, platform, game_id
```

The provider never launches anything. It reports `provider_id = "dolphin"`, one record per ROM,
and no `DetectedTarget`.

## Launch

```text
extension session (target game)
  ├─ resolve GameNode entry + ROM file handle (approved files)
  ├─ find/register the Dolphin runtime (approved executable handle)
  ├─ bind ROM content → runtime (content binding)
  ├─ propose recipe  ──► host review + user consent (once)
  └─ request recipe  ──► host launches runtime, supervises process tree
                          └─ exit detected by the host, session/playtime recorded
```

The recipe carries only opaque host handles: no executable path, no argv, no environment. The
runtime's argument template is `-b -e {content}`. The host substitutes the approved ROM path as a
single argv element, so the plugin cannot smuggle shell text or another file.

`-b` (batch) makes Dolphin exit when the game ends. Without it the session ends when the emulator
window closes, not when the game stops.

## Process exit

The host's launch supervisor owns the process tree, not the plugin:

- it acquires the shared-resource leases before spawning;
- it tracks the main process and descendants;
- it transitions the launch job to `Exited` when the tree is gone;
- playtime is attributed from that session.

A plugin that polls the emulator or spawns it directly would bypass all of this and is out of
scope.

## Identity (current representation and planned separation)

- plugin id: `org.example.gamenode.dolphin` / `org.example.gamenode.dolphin-launch`
- provider id: `dolphin`
- provider-local record ids today: the six-character disc id (`GZLE01`, `RMCE01`) when readable, otherwise
  `dolphin-<stable hash of the absolute path>`
- `install_dir`: the folder containing the ROM, so collections group together
- fields: `rom` (file name), `platform` (`gamecube`/`wii`/`unknown`), `game_id` (empty when
  unknown)

The current `fields.game_id` is a disc ID, not a GameNode catalog ID. Rename this metadata to
`disc_id` with a compatibility reader for existing entries. Keep the host-selected GameNode
`game_id` and `installation_id` for resource and launch requests. The current host requires a
catalog-matched game for content registration and custom launch; unresolved matches must be
reported rather than substituting the disc ID. Define distinct content identities for copies,
revisions and disc numbers before changing discovery deduplication.

## Metadata and persistence (planned)

The provider owns title resolution and catalog matching. Prefer local Dolphin title mappings:
`Load/wiitdb.txt`, or `Load/titles.txt` when the former is empty, then
`Sys/wiitdb-<language>.txt` with English fallback. If unavailable, use readable ROM metadata and
finally the file name. Discover the actual Dolphin data locations per installation; do not assume
the configuration directory is the system-data directory. Keep parsing bounded and offline.

These databases resolve IDs to titles, not complete synopses or GameNode catalog identities.
Preserve source and language with cached metadata, and keep disc ID and GameTDB ID distinct where
they differ. Descriptions from ROM metadata are optional; full catalog descriptions are separate
enrichment. Prefer these text files over coupling the MVP to Dolphin's binary GameListCache.

Use GameNode `StorageGet/Put/Delete` with explicit persistent durability for small versioned
records. Cache discovered games only, not an entire external catalog. The provider stores metadata
and auxiliary matching state; authoritative associations go through `MatchingSearch/Assign` and
the library. Preserve user-selected matches and require resolution of ambiguous candidates.

Storage is private to plugin ID, publisher fingerprint and account profile. The two components
cannot read each other's keys even in a bundle. The extension reads the selected library entry
through the host and stores its own durable runtime/content/recipe IDs and revisions. Revalidate
those references before reuse. Never persist session handles, credentials or user-intent tokens.
Handle denied persistence, quota exhaustion, cleanup and revision conflicts explicitly.

Sources: Dolphin's [TitleDatabase.cpp](https://github.com/dolphin-emu/dolphin/blob/master/Source/Core/Core/TitleDatabase.cpp)
and [GameFile.cpp](https://github.com/dolphin-emu/dolphin/blob/master/Source/Core/UICommon/GameFile.cpp),
inspected on 2026-11-20. Their behavior is a reference for the resolver, not a stable plugin API.

## Permissions

Provider manifest:

- `provider.context.read` (required): authenticate and read the provider context.
- `library.discovery` (optional): report the games.

Extension manifest:

- `game.custom_launch` (required): propose/request the recipe.
- `game.runtime.read/create/update/delete` (optional): register and maintain Dolphin.
- `files.read` (optional): obtain the approved ROM and executable handles.
- `library.read` (optional): resolve the selected game entry.

Nothing here grants filesystem sandboxing; the plugin is an ordinary process with the user's OS
permissions. The API boundary is consent and least authority, not a sandbox.

Planned permission additions must be applied to capabilities and manifests together with their
implementation: extension `library.discovery` for content registration; provider `library.read`,
`matching.read` and `matching.write` for catalog association; each component's `storage.read`,
`storage.write`, `storage.persist` and `storage.delete` where used. These additions are not yet
declared by the current manifests. Metadata overrides, if implemented, require their own grant.

## Boundaries

- No path, argv or environment override in a recipe request.
- No process spawn from the plugin.
- No internal Core/platform crates: only `gamenode-plugin-sdk` (and `async-trait`/`tokio`).
- No secrets in logs, package files or storage.
