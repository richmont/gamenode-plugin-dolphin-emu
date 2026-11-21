# Spec Delta

## Purpose

Launch the selected Dolphin game through GameNode's approved runtime recipes and host-owned process supervision.

## ADDED Requirements

### Requirement: Selected-game launch authority
The extension SHALL act only on the host-selected game and installation, using a host-approved executable and ROM reference. Content registration SHALL require discovery and file grants. A runtime name or shared catalog visibility alone SHALL not establish execution authority.

#### Scenario: Missing approval or match
- **WHEN** the selected game has no catalog match or the required runtime/file authority is missing
- **THEN** launch stops with actionable guidance and the plugin starts no process.

#### Scenario: Readiness-only session
- **WHEN** the host starts an initialization, check or maintenance-readiness session without a launch action
- **THEN** the extension does not request game execution.

### Requirement: Immutable consented execution
The extension SHALL submit immutable runtime/content recipes and request only approved revisions, forwarding the current host-issued intent unchanged. Execution requests SHALL contain no replacement executable, arguments, working directory or environment. Denial SHALL never fall back to direct process spawning.

#### Scenario: First use
- **WHEN** the user first launches a matched game with approved file access
- **THEN** the host reviews the recipe before execution and a single-use intent is not consumed twice.

#### Scenario: Revoked or stale approval
- **WHEN** a grant is revoked, an intent expires or the approved plan changes
- **THEN** execution is refused or requires fresh host approval without an alternate launch route.

### Requirement: Durable launch references
The extension SHALL retain durable runtime, content and recipe identities through its own persistent Storage namespace, revalidate them against current host state and keep their revisions separate. Session handles and intents SHALL be reacquired rather than persisted.

#### Scenario: Repeated play after restart
- **WHEN** the same game is launched again with unchanged approved state
- **THEN** valid bindings and recipes are reused without duplicate registration and the new action supplies its own intent.

#### Scenario: Missing or changed resource
- **WHEN** a stored reference points to removed content or a changed runtime
- **THEN** the extension reconciles current state and obtains required approval before launching.

### Requirement: Host-owned process lifecycle
The plugin SHALL delegate process start, supervision, session completion and playtime attribution to GameNode. It SHALL not spawn, signal, poll or reap Dolphin itself.

#### Scenario: Game ends
- **WHEN** the supervised Dolphin process tree exits after an approved launch
- **THEN** GameNode completes the launch session and records its playtime for the selected game.
