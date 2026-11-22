# Architecture

## Why two packages

A GameNode package is either a **provider** or an **extension**; it cannot be both. The split here
is not cosmetic, it matches two different host capabilities:

| Package | Kind | Job | Grant |
| --- | --- | --- | --- |
| `gamenode-dolphin-emu-provider` | provider | Scan Dolphin's game folders and report GameCube/Wii games | `library.discovery` |
| `gamenode-dolphin-emu-launch` | extension | Ask the host to approve Dolphin, bind the ROM and run an approved launch recipe | `game.launch` + `game.custom_launch` + runtime/file/library/storage grants |

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

The provider never launches anything. It reports `provider_id = "dolphin-emu"`, one record per ROM,
and no `DetectedTarget`.

Homebrew executables (`.dol`, `.elf`) come through the same scan. They carry no disc header, so the
record keeps the path-derived `dolphin-emu-<hash>` id, reports `platform` `unknown` and never
borrows an id or strips a prefix from its file name. Discovery does not guess GameCube or Wii,
because Dolphin homebrew exists for both; the launch path is unchanged.

## Launch

```text
extension session (target game)
  ├─ read the selected library entry (game/installation identity, title)
  ├─ request the approved installation folder and select the ROM file
  ├─ reuse or ask the host picker for an approved Dolphin runtime
  ├─ bind ROM content → runtime (stable content binding)
  ├─ reuse an approved recipe or propose one ──► host review + user consent
  └─ request the approved recipe ──► host launches runtime, supervises process tree
                          └─ exit detected by the host, session/playtime recorded
```

The recipe carries only opaque host handles: no executable path, no argv, no environment. The
runtime's argument template is `-b -e {content}`. The host substitutes the approved ROM path as a
single argv element, so the plugin cannot smuggle shell text or another file.

GameNode starts an extension package as a one-shot process for the user's **Play** action. The
host issues the target and intent; the plugin performs the resource calls and then keeps the
session open until the host's launch job has passed its approval fence. A readiness startup has no
intent and exits without launching. The plugin never starts Dolphin itself.

The host's `RuntimesRequestAccess` picker owns the executable: the request carries the runtime
spec and text hints (`dolphin-emu`, `dolphin-emu.desktop`) but no handle, and the reply carries no
path or handle. A changed executable invalidates the approval and the host asks again.

`LaunchState` in private Storage contains only `runtime_id`, `content_id`, `recipe_id` and their
revisions, plus the selected ROM's name, size and modification time. It never contains a file
handle, executable handle, path, argv, environment or user intent. A repeated Play rereads the
entry, revalidates the runtime and file listing, reuses valid references and calls
`GamesRequestCustomLaunch` with the current intent. A changed ROM (name, size or modification
time), runtime or content invalidates the old recipe and requires a fresh review.

Host API limits observed by this implementation:

- `LibraryEntry` has no `rom`, `platform` or `title` field. The entry name is the provider's cleaned
  file name, so the ROM is selected from the approved installation listing by normalized title,
  or accepted only when the folder has exactly one Dolphin ROM. Ambiguity is refused.
- The runtime/content platform field is the host OS, not the console. The runtime accepts the
  generic content kind `rom`; Dolphin loads both GameCube and Wii images.
- The content approval has no digest. Size and modification time detect common in-place changes;
  a replacement that preserves both is not distinguishable without a host content-identity API.

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

- plugin id: `dev.richmont.gamenode.dolphin-emu` / `dev.richmont.gamenode.dolphin-emu-launch`
- provider id: `dolphin-emu`
- provider-local record ids today: the six-character disc id (`GZLE01`, `RMCE01`) when readable, otherwise
  `dolphin-emu-<stable hash of the absolute path>`
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

- `game.launch` (required): let GameNode start a one-shot extension session for Play.
- `game.custom_launch` (required): propose/request the recipe.
- `game.runtime.read/create` (optional): find or ask the host picker for Dolphin.
- `files.read` (optional): obtain the approved installation folder and ROM handle.
- `library.read` + `library.read.all` (optional): resolve the selected game entry. GameNode exposes
  another provider's selected entry to an extension only under the whole-library read grant; the
  extension reads only the target installation.
- `library.discovery` (optional): register the ROM content binding.
- `storage.read/write/persist` (optional): remember durable runtime/content/recipe identities.
- `notifications.publish` (optional): explain a refused or failed launch.

Nothing here grants filesystem sandboxing; the plugin is an ordinary process with the user's OS
permissions. The API boundary is consent and least authority, not a sandbox.

Provider-side catalog association (`library.read`, `matching.read`, `matching.write`) and
provider Storage caches are still planned; they are not declared by the current provider manifest.
Metadata overrides, if implemented, require their own grant.

## Boundaries

- No path, argv or environment override in a recipe request.
- No process spawn from the plugin.
- No internal Core/platform crates: only `gamenode-plugin-sdk` (and `async-trait`/`tokio`).
- No secrets in logs, package files or storage.

## Durable shared catalog (implemented)

After worker-thread scanning, the provider asynchronously calls public SDK
`host.resources().ok_or(Error::Protocol)?` and `Resources::data_publish(key, 0, request)`.
The SDK supplies mutation/session context; the plugin never constructs it manually.
Dataset and schema are `dev.richmont.gamenode.dolphin-emu.catalog.v1`, version 1, `PERSISTENT`, TTL 0.
The string fields are `title`, `platform` and `content_id`, plus `disc_id` when a disc header or a
`GZLE01 - Title` file name supplied one. Homebrew executables publish no `disc_id` field at all
rather than an empty string. Platform is `gamecube`, `wii`, or `unknown` when the current scanner
cannot identify it. Title comes from the disc header (plain images and RVZ) when readable, else from the cleaned filename; WBFS/WIA still fall back to the file name. Dolphin's title database is not read yet.
There is no fabricated GameNode catalog ID: matching and local title database resolution remain
pending.

`resource_id` and `content_id` use the disc ID when known, otherwise the scanner's stable opaque
file ID. Copies, revisions and multiple discs sharing a six-character ID collapse into one
catalog row, distinct from installation identity. The first record in the scanner's sorted result
wins within a scan. Repeated scans replace only this authenticated producer's row and increase
its host revision; same-identity rows from another publisher remain separate. Unknown-file IDs
are path-derived hashes, not content checksums; moving such a file changes its identity. A rescan
is not a deletion or availability proof for older rows. Host retention/uninstall policy owns cleanup.

Only these metadata fields are published; discovery paths and filenames, handles, argv,
environment, credentials and session data are excluded. Provenance belongs to the host. Each
publication has a fresh observation/idempotency ID, observation time and positive sequence;
expected revision remains 0 under this API's append/replace contract. No automatic retry occurs.

Provider `data.publish` and `storage.persist` use `own-provider` scope and remain optional:
ROM discovery works without catalog publication. A host denial logs only its status code and SDK
safe category, returns discovery records, and never retries as SESSION_ONLY. Publication stops
at the first error in a scan. Persistent records are private by default and consumers still need
host sharing approval. Their lifetime is independent of the provider session, subject to retention
and revocation. This shared pool is separate from private Storage bookkeeping planned elsewhere.

SDK reference: GameNode integration `971a8a8` for the durable catalog; the launch session in
[launch-session-evidence.md](launch-session-evidence.md) uses `cc667cb` (approved runtime picker).
See README for exact signed-bundle commands and
[delivery evidence](durable-catalog-evidence.md). Production consumer/revocation/UI acceptance
belongs to the main-repo integration session, not the independent reference-host fixture.
