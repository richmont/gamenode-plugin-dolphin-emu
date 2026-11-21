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

## Identity

- plugin id: `org.example.gamenode.dolphin` / `org.example.gamenode.dolphin-launch`
- provider id: `dolphin`
- game ids: the six-character disc id (`GZLE01`, `RMCE01`) when readable, otherwise
  `dolphin-<stable hash of the absolute path>`
- `install_dir`: the folder containing the ROM, so collections group together
- fields: `rom` (file name), `platform` (`gamecube`/`wii`/`unknown`), `game_id` (empty when
  unknown)

The game id is what Dolphin uses for saves and covers, so it is worth preserving for compressed
containers too (see `TASKS.md`).

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

## Boundaries

- No path, argv or environment override in a recipe request.
- No process spawn from the plugin.
- No internal Core/platform crates: only `gamenode-plugin-sdk` (and `async-trait`/`tokio`).
- No secrets in logs, package files or storage.
