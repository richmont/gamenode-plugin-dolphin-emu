# Proposal

## Why

The provider discovers ROMs and the launch extension now implements the selected-game session
(approved runtime, ROM content binding, recipe and execution request) against the desktop
runtime-picker contract. The remaining work in this change is catalog identity/matching, local
title resolution, bounded persistent caches and native acceptance.

## What Changes

- Separate disc metadata from GameNode catalog, installation and content identities, with legacy
  field compatibility and explicit handling of copies, revisions and multiple discs.
- Resolve titles from Dolphin's local text databases, then ROM metadata/file names; establish
  authoritative associations through GameNode Matching rather than a private cache alone.
- Publish a minimal persistent shared disc catalog through DataPublish, with optional publication
  and retention grants; keep private bookkeeping separate.
- Use GameNode persistent Storage for bounded metadata caches and durable launch references,
  respecting separate provider/extension namespaces and account scope.
- Implement selected-game runtime/content/recipe setup, consent and repeat launches, with
  host-owned process supervision and no direct emulator spawning.
- Deliver two signed Linux x86_64 MVP packages and a signed two-component development bundle. Plan
  Windows releases, possible macOS/additional architectures and native bundle acceptance
  separately, declaring only shipped manifest targets.

## Capabilities

### New Capabilities

- `dolphin-metadata`: disc identity, local title resolution, catalog matching and bounded persistent caches.
- `dolphin-emu-launch`: approved runtime/content recipes, durable references and supervised execution.
- `dolphin-distribution`: verifiable component packages and platform acceptance.

### Modified Capabilities

None. This repository has no pre-existing OpenSpec specifications.

## Impact

Changes will affect `provider/src/`, `launch/src/`, their manifests and tests, and project docs.
Only the public SDK is allowed; it currently comes from the sibling GameNode checkout. The host
runtime picker (`add-approved-runtime-picker`) resolves the executable-approval dependency. The
extension still needs the selected library entry's provider fields (`rom`/`platform`/`title`),
which `LibraryEntry` does not expose; it selects the ROM from the approved installation listing
and refuses ambiguity. Native Linux acceptance and the Windows handoff remain open.

Full synopses, covers, save backup/restore, binary GameListCache parsing, host bundle implementation and
non-Linux native certification are outside this MVP change and remain in the root backlog.
