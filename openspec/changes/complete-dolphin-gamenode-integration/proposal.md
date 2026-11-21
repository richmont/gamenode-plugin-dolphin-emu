# Proposal

## Why

The provider discovers ROMs, but the launch extension currently returns success without requesting
execution. A usable Linux MVP needs reliable catalog identity, persistent state and an approved
Dolphin runtime connected to GameNode's supervised launch flow.

## What Changes

- Separate disc metadata from GameNode catalog, installation and content identities, with legacy
  field compatibility and explicit handling of copies, revisions and multiple discs.
- Resolve titles from Dolphin's local text databases, then ROM metadata/file names; establish
  authoritative associations through GameNode Matching rather than a private cache alone.
- Use GameNode persistent Storage for bounded metadata caches and durable launch references,
  respecting separate provider/extension namespaces and account scope.
- Implement selected-game runtime/content/recipe setup, consent and repeat launches, with
  host-owned process supervision and no direct emulator spawning.
- Deliver two signed Linux x86_64 MVP packages. Plan Windows releases, possible macOS/additional
  architectures and host-dependent bundles separately, declaring only shipped manifest targets.

## Capabilities

### New Capabilities

- `dolphin-metadata`: disc identity, local title resolution, catalog matching and bounded persistent caches.
- `dolphin-launch`: approved runtime/content recipes, durable references and supervised execution.
- `dolphin-distribution`: verifiable component packages and platform acceptance.

### Modified Capabilities

None. This repository has no pre-existing OpenSpec specifications.

## Impact

Changes will affect `provider/src/`, `launch/src/`, their manifests and tests, and project docs.
Only the public SDK is allowed; it currently comes from the sibling GameNode checkout. Runtime
executable approval is a host dependency to verify before implementation; extending that host is
separate work if its current UI/API cannot provide the required approved handle and ownership.
Caps and permissions must change with implementation, not merely with this planning update.

Full synopses, covers, save backup/restore, binary GameListCache parsing, bundle implementation and
non-Linux native certification are outside this MVP change and remain in the root backlog.
