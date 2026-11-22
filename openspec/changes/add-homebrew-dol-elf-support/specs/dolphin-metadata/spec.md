# Spec Delta

## ADDED Requirements

### Requirement: Homebrew executable discovery
The provider SHALL report GameCube and Wii homebrew executables with `dol` or `elf` extensions from
the same bounded scan used for disc images, matching extensions case-insensitively.

#### Scenario: Homebrew executable found
- **WHEN** a scanned root contains `240pSuite.dol` or `homebrew.elf`
- **THEN** discovery reports one record per file, titled with the cleaned file name

#### Scenario: Unsupported file ignored
- **WHEN** a scanned root contains `notes.dol.txt` or a file with another extension
- **THEN** the file is not reported and the scan bounds are unchanged

### Requirement: No invented homebrew identity
For a file without a disc header, the provider MUST NOT invent a disc ID or platform. It SHALL use
the existing path-derived scanner identity as the record ID and report `platform` as `unknown`.

#### Scenario: DOL without a disc header
- **WHEN** a `.dol` file starts with section offsets instead of a six-character disc ID
- **THEN** the record carries no `game_id` and keeps the `dolphin-emu-` scanner identity

#### Scenario: ELF executable
- **WHEN** an `.elf` file starts with the ELF magic instead of a disc header
- **THEN** the record carries no `game_id` and reports `platform` `unknown`

#### Scenario: Homebrew file moved
- **WHEN** a homebrew file is moved or renamed between scans
- **THEN** it is discovered under a new identity and no existing catalog row is overwritten

### Requirement: Conditional catalog disc ID
The durable catalog record SHALL include a `disc_id` field only when a disc ID exists, and MUST
omit the field rather than publish an empty string.

#### Scenario: Headerless publication
- **WHEN** the provider publishes a catalog record for a headerless executable
- **THEN** the payload has no `disc_id` field and keeps title, platform and content identity

#### Scenario: Disc publication unchanged
- **WHEN** the provider publishes a record for a disc image with a readable ID
- **THEN** the payload still contains that `disc_id` and the existing dataset and schema

### Requirement: Homebrew launch compatibility
Homebrew records SHALL use the same discovery fields and approved launch path as disc records, so
the existing batch recipe applies and the plugin still starts no process.

#### Scenario: Approved homebrew launch
- **WHEN** the user approves a homebrew record and plays it
- **THEN** the host launches Dolphin through the existing recipe and the plugin spawns nothing
