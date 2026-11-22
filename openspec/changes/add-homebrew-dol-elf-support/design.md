# Design

## Context

`provider/src/roms.rs` owns discovery: one bounded, recursive scan over Dolphin's configured roots
plus `DOLPHIN_PLUGIN_ROOTS`, a case-insensitive extension filter (`iso`, `gcm`, `rvz`, `wbfs`,
`wia`, `ciso`, `gcz`), a six-byte disc header read, a filename ID heuristic, and a
path-derived FNV-1a identity used whenever no disc ID is available. `provider/src/catalog.rs`
turns a discovery record into the durable `dev.richmont.gamenode.dolphin-emu.catalog.v1` payload.
`launch/src/lib.rs` binds content as kind `rom` and proposes `dolphin-emu -b -e <content>`.

Observed fixture facts (240p Test Suite, GPL-2.0-or-later, kept outside Git):

- `GameCube-240pSuite-1.9.dol` starts with `00 00 01 00 00 00 00 00` (section offsets), so the disc
  header read yields nothing and the filename has no leading six-character ID.
- `240pTest-GC.iso` (1.5) and `GameCube-240pSuite-1.9.iso` (1.9) both carry the synthetic disc ID
  `GBLPGL` and the disc magic at `0x1C`; the ISO already boots in Dolphin 2609
  (`docs/evidence/240p-test-suite-dolphin-2609.png`).

See `proposal.md` for motivation and `specs/dolphin-metadata/spec.md` for requirements.

## Goals / Non-Goals

**Goals**

- Make homebrew executables discoverable, launchable and catalogued without weakening the honesty
  of disc identity.
- Keep the change local to the provider's extension, identity and payload logic.

**Non-Goals**

- Converting DOL/ELF into disc images, or reading Dolphin's `apps/` layout or WAD metadata.
- Parsing DOL section headers, Elf32 headers or Wii NAND state to infer a platform.
- Content hashing or any new identity scheme; installation matching stays with host Matching.
- Changing manifests, permissions, bundle output or the launch recipe.

## Decisions

### Extend the existing extension list

Add `dol` and `elf` to `ROM_EXTENSIONS` and reuse the current predicate. The scan stays bounded by
depth and entry count, and only eight bytes of each candidate are ever read.

*Alternative rejected:* a second scan pass for executables. It doubles directory walks and
complicates the entry budget for no behavioural gain.

### No disc ID derived from an executable's file name

`read_game_id` already rejects DOL offsets and ELF magic, but `id_from_filename` would still turn
`GZLE01 demo.dol` into a disc ID. For `dol`/`elf` the provider therefore skips the filename
heuristic entirely and falls back to the path-derived `dolphin-emu-<hash>` identity.

*Alternative rejected:* keeping the heuristic for every extension. It would fabricate a disc
identity for a file that has none, violating the "no invented identity" requirement.

*Alternative rejected:* hashing file contents for a stable identity. It costs a full read per file
and still changes nothing for the ISO+DOL duplication case, which only the user can resolve.

### Platform stays `unknown`

`platform_of` maps `G`/`D` to GameCube and `R`/`S`/`W` to Wii; with no ID it reports `unknown`.
Executables do not record their target, and Wii homebrew DOLs are common, so inferring `gamecube`
from the extension would mislabel real files.

*Alternative rejected:* defaulting to `gamecube` because most homebrew DOLs are GameCube builds.

Consequence to verify during acceptance: the extension registers runtime formats
`gamecube-rom` and `wii-rom`, so the host must still bind an `unknown`-platform record. If the host
rejects it, that is a host dependency to report, not a reason for the plugin to guess.

### Catalog payload omits an unknown disc ID

`catalog::publication` currently reads `fields["game_id"]` with an empty default and always emits
`disc_id`. Change it to emit the field only when the ID is non-empty. Dataset, schema version,
durability, observation fields and `resource_id`/`content_id` are untouched, so an unknown-ID
record still replaces only its own producer row.

*Alternative rejected:* publishing `disc_id: ""`. Consumers cannot distinguish "no disc ID" from a
malformed producer, and an empty string reads like a value.

### No change to the launch extension

`content_binding` already uses kind `rom`, and the recipe already passes the ROM path to `-b -e`.
Executables follow the same path, including host-owned process supervision and exit detection.

## Risks / Trade-offs

- [Host rejects `unknown` platform when binding a runtime] → verify with a real host session
  before claiming acceptance; record the outcome and report the host gap instead of faking a
  platform.
- [A Wii DOL needs NAND/system files and may fail where a GameCube DOL succeeds] → use the
  GameCube build for the acceptance fixture and document the Wii limitation.
- [Path identity changes on move or rename] → document it; discovery is not a content identity and
  the host keeps the existing install/match history.
- [A user keeping both the ISO and the DOL of the same homebrew sees two entries] → document that
  the two forms are distinct identities and that only the disc form collapses copies.
- [An `.elf` debug build can be large] → only the eight-byte header is read; the scan budget is
  unchanged.

## Migration Plan

Additive and backward compatible: no manifest, permission, dataset or schema change. Existing disc
records and their revisions are untouched. Rolling back means removing the two extensions; homebrew
entries stop being rediscovered, and their catalog rows remain subject to host retention and
uninstall policy.

## Open Questions

None that would change the requirements or the task breakdown.
