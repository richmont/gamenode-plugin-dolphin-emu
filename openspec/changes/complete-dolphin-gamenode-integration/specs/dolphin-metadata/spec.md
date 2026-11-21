# Spec Delta

## Purpose

Resolve Dolphin disc metadata into reliable library entries without confusing disc identity with GameNode catalog identity.

## ADDED Requirements

### Requirement: Separate identities
The plugin SHALL preserve disc identity separately from catalog, installation and content identities, including readable revision and disc number. Legacy disc metadata SHALL remain readable without becoming an RPC catalog ID.

#### Scenario: Legacy entry
- **WHEN** an entry contains `GZLE01` in legacy `fields.game_id`
- **THEN** it is interpreted as disc metadata and launch uses only the host-selected catalog ID.

#### Scenario: Multiple copies or discs
- **WHEN** files share a disc ID but represent distinct copies, revisions or disc numbers
- **THEN** discovery preserves selectable content without deleting existing catalog history.

### Requirement: Offline title resolution
The provider SHALL resolve titles from local Dolphin user mappings, then localized system mappings with English fallback, then readable ROM metadata and file name. It SHALL bound parsing and record the selected source and language; missing or malformed databases SHALL not prevent discovery.

#### Scenario: User title override
- **WHEN** a user title mapping and a system mapping both contain a discovered ID
- **THEN** the user title wins without a network request.

#### Scenario: Database unavailable
- **WHEN** no usable local mapping exists
- **THEN** the provider uses readable ROM metadata or the file name and continues discovery.

### Requirement: Authoritative catalog association
The provider SHALL use GameNode matching services to establish catalog associations, preserve user-selected matches and leave ambiguous candidates unresolved. Private cached associations SHALL not replace the library's current authoritative state.

#### Scenario: Ambiguous catalog candidates
- **WHEN** several candidates remain plausible
- **THEN** the game remains unmatched with guidance to resolve the choice and no catalog ID is fabricated.

#### Scenario: User changes a match
- **WHEN** a cached association differs from the current user-selected library match
- **THEN** the user-selected association is preserved and the cache is reconciled.

### Requirement: Host-managed persistent state
The plugin SHALL use GameNode persistent Storage for bounded versioned cache records and auxiliary state, respecting namespace isolation, quotas and optimistic revisions. It SHALL exclude session handles, credentials and intent tokens and recover safely from cleared or incompatible cache records.

#### Scenario: Restart and isolation
- **WHEN** the provider restarts in the same account with intact persistent storage
- **THEN** it can reuse valid cached metadata, while the extension and other accounts cannot read its private keys.

#### Scenario: Storage failure
- **WHEN** persistence is denied, a quota is exceeded or a revision conflicts
- **THEN** the plugin does not report durable success and handles the failure without overwriting newer state.

#### Scenario: Cache removed
- **WHEN** the user clears plugin storage
- **THEN** metadata can be rebuilt without deleting host library matches or history.
