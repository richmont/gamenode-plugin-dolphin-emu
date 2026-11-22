# Proposal

## Why

Dolphin users run GameCube and Wii homebrew as bare executables (`.dol`, `.elf`) loaded through
Swiss, an SD media launcher or the Homebrew Channel, but discovery only reports disc images. Those
files are therefore invisible to the GameNode library and cannot use the approved launch flow. A
real homebrew fixture also lets native acceptance prove discovery and launch without depending on
a copyrighted game image.

## What Changes

- Report `.dol` and `.elf` files from the same bounded, case-insensitive scan that already finds
  disc images; no new roots, permissions or crates.
- Report headerless executables honestly: a stable path-derived scanner identity, `platform`
  `unknown`, the cleaned file name as the title, and no invented disc ID or platform.
- Publish the durable catalog record without a `disc_id` field when the file has no disc header,
  instead of sending an empty string; `resource_id` and `content_id` keep the opaque identity.
- Keep the approved launch recipe unchanged: `dolphin-emu -b -e <content>` already accepts
  executables, and the plugin still never starts or supervises a process.
- Document that the identity is derived from the path, so moving or renaming a homebrew file
  creates a new identity and never overwrites an existing catalog row.
- No breaking changes; disc-image discovery, payloads and permissions are unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `dolphin-metadata`: discovery scope and identity rules gain headerless homebrew executables, and
  the durable catalog payload becomes conditional on a known disc ID. Its main specification is
  still in flight in `complete-dolphin-gamenode-integration`, so this change adds requirements to
  the same capability instead of introducing a near-duplicate one.

## Impact

Affects `provider/src/roms.rs` (extension set, identity, platform), `provider/src/catalog.rs`
(conditional `disc_id`), their tests, and `README.md`, `TASKS.md`, `docs/architecture.md` and
`docs/protocol-notes.md`. No manifest, permission, bundle or `launch/src/` change is expected.

Risks: the host may reject an `unknown` platform when binding the runtime, and a Wii DOL may need
NAND/system files. Both are verification items, not reasons to fake a platform. A user who keeps
both an ISO and a DOL of the same homebrew will see two entries, because their identities differ.

Validation fixture for native acceptance: 240p Test Suite 1.9
`GameCube-240pSuite-1.9.dol` (GPL-2.0-or-later), kept outside Git with its version and hash. The
boot behaviour of the suite's disc image in Dolphin 2609 is already recorded in
`docs/evidence/240p-test-suite-dolphin-2609.png`, and the change is expected to work with the
`.dol` build as well.
