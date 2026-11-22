# Spec Delta

## Purpose

Distribute verifiable Dolphin provider and launch packages with accurate platform declarations and native acceptance evidence.

## ADDED Requirements

### Requirement: Installable Linux MVP
The MVP SHALL provide separately signed provider and extension packages for Linux x86_64 using valid publisher keys and the host's package verification. It SHALL not depend on bundle availability.

#### Scenario: Component installation
- **WHEN** the user imports both MVP packages and reviews their permissions
- **THEN** GameNode can install them and expose discovery and the extension launch action.

### Requirement: Accurate target declarations
Every distributed OS, architecture and ABI SHALL have a corresponding binary and target entry in each component manifest. Windows is a planned release; macOS and additional architectures remain optional follow-ups. Unsupported targets SHALL not be advertised as working.

#### Scenario: Adding a Windows build
- **WHEN** Windows packages are prepared for release
- **THEN** both manifests declare matching Windows targets and native transport and launch evidence is recorded separately from cross-build success.

### Requirement: End-to-end acceptance evidence
MVP acceptance SHALL record native Linux discovery, catalog association, approved launch, repeated Play and session/playtime completion for GameCube and Wii. Unit tests alone SHALL not certify this workflow.

#### Scenario: Launch fixture passes without native run
- **WHEN** SDK fixtures pass but no native Dolphin run has been performed
- **THEN** the end-to-end acceptance task remains incomplete.

### Requirement: Signed development bundle
The delivery SHALL include a development-signed bundle `dev.richmont.gamenode.dolphin-emu.bundle` version `0.1.0` with exactly the provider and launch component packages and one common publisher. Private keys and generated archives SHALL remain outside version control.

#### Scenario: Artifact verification
- **WHEN** the bundle is produced
- **THEN** both component signatures, the bundle signature, component hashes and publisher equality verify, while native UI acceptance remains separately recorded.
