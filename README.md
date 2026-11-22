# gamenode-plugin-dolphin-emu

A standalone GameNode plugin for **GameCube and Wii games managed by Dolphin**. The repository is
one workspace that builds two packages (discovery provider and launch extension); the intended
distribution is a single signed `.gnbundle` containing both signed components. It is a separate
project on purpose: it exercises the public SDK, the package/consent conventions and the plugin
documentation as an external developer would see them.

## What works today

- **Discovery** (`provider/`): reads Dolphin's own `ISOPath0..N` entries from `Dolphin.ini`
  (Linux, Flatpak, Windows, macOS) plus `DOLPHIN_PLUGIN_ROOTS`, scans the folders recursively and
  reports `.iso`, `.gcm`, `.rvz`, `.wbfs`, `.wia`, `.ciso`, `.gcz`, `.dol` and `.elf` games. Plain
  discs expose their six-character game id (`GZLE01`, `RMCE01`); compressed containers fall back to
  a `GZLE01 - Title` file name or a stable path hash. Homebrew executables (`.dol`, `.elf`) have no
  disc header, so they keep a path-derived id and report `unknown` platform instead of a guessed
  one. Hidden folders, symlinks, deep trees and huge collections are bounded.
- **Durable catalog**: after discovery, publishes schema-1 records to
  `dev.richmont.gamenode.dolphin-emu.catalog.v1` using SDK `data_publish` and `PERSISTENT`. Optional
  `data.publish` and `storage.persist` grants enable publication; refusal is reported without
  discarding discovery or downgrading to session-only.
- **Launch recipe** (`launch/`): builds the Dolphin runtime (`-b -e <rom>`) and the immutable
  custom-launch recipe/content binding. The host supervises the process tree, so it detects when
  the game ends and attributes playtime; the plugin never starts a process itself.
- Tests: `cargo test` covers game-id parsing, compressed-container fallback, homebrew executable
  identity, Dolphin config parsing, recursive scanning/hidden folders, runtime arguments and
  recipe immutability. Explicit depth and entry-limit regression tests remain pending.

## What is still a task

`launch/` does not yet perform the session flow (find or register the Dolphin runtime, bind the
ROM handle, propose/request the approved recipe). `TASKS.md` lists it as the first task with the
exact SDK calls and permissions. The plugin intentionally does nothing unsafe while that is
pending.

The MVP targets Linux x86_64. Windows releases are planned; macOS and additional architectures
are possible follow-ups. Each supported OS/architecture/ABI must have a matching binary and a
`targets` entry in both component manifests. Configuration discovery on a platform does not
constitute native launch acceptance. Bundle support is independent of platform support.

The agreed integration design uses GameNode's persistent Storage endpoints for bounded metadata
caches and launch bookkeeping, and its Matching API for authoritative catalog associations.
Dolphin disc IDs remain separate from GameNode catalog IDs. Title resolution will reuse Dolphin's
local title databases before falling back to ROM metadata and file names; it is not implemented
yet. See [Architecture](docs/architecture.md) and [Tasks](TASKS.md).

## Build

The public SDK is not on crates.io yet. This workspace points at the reviewed sibling checkout
`../game-node-desktop/crates/plugin-sdk`; after the SDK is published, replace it with
`gamenode-plugin-sdk = "=0.1.0"` (see `Cargo.toml`).

```sh
CARGO_TARGET_DIR=/data/DADOS_LINUX/geral/area_troca_rust/target-dolphin-plugin CARGO_NET_OFFLINE=true CARGO_INCREMENTAL=0 \
  cargo test
CARGO_TARGET_DIR=/data/DADOS_LINUX/geral/area_troca_rust/target-dolphin-plugin CARGO_NET_OFFLINE=true CARGO_INCREMENTAL=0 \
  cargo build --release
```

## Build, sign and bundle (Linux x86_64)

SDK/host reference: `971a8a8acc1084e7a9c9e1163260ad6110df8b34`. Run from this repository.
The checked-in public key is **development-only**, not a release publisher identity. The private
key is outside Git. To regenerate it, run the following keygen command once and replace
`publisher.public_key` in both manifests with its printed public key, keeping the same key_id.
Never overwrite a key whose fingerprint is already pinned without a new user review.

```sh
mkdir -p /tmp/dolphin-keys
python3 ../game-node-desktop/scripts/plugin-package.py keygen --development --private-key /tmp/dolphin-keys/catalog-dev.pem
```

Regenerate notices from the locked runtime dependency graph before packaging:

```sh
env CARGO_TARGET_DIR=/data/DADOS_LINUX/geral/area_troca_rust/target-dolphin-plugin CARGO_NET_OFFLINE=true CARGO_INCREMENTAL=0 cargo metadata --locked --format-version 1 > /tmp/dolphin-metadata.json
python3 scripts/dependency-notices.py /tmp/dolphin-metadata.json
```

With the matching key already present, the exact packaging commands are:

```sh
env CARGO_TARGET_DIR=/data/DADOS_LINUX/geral/area_troca_rust/target-dolphin-plugin CARGO_NET_OFFLINE=true CARGO_INCREMENTAL=0 cargo build --release --workspace
mkdir -p provider/package/bin/linux-x64 launch/package/bin/linux-x64 dist
cp /data/DADOS_LINUX/geral/area_troca_rust/target-dolphin-plugin/release/gamenode-dolphin-emu-provider provider/package/bin/linux-x64/plugin
cp /data/DADOS_LINUX/geral/area_troca_rust/target-dolphin-plugin/release/gamenode-dolphin-emu-launch launch/package/bin/linux-x64/plugin
python3 ../game-node-desktop/scripts/plugin-package.py pack --source provider/package --private-key /tmp/dolphin-keys/catalog-dev.pem --output dist/dolphin-emu-provider.gnplugin
python3 ../game-node-desktop/scripts/plugin-package.py pack --source launch/package --private-key /tmp/dolphin-keys/catalog-dev.pem --output dist/dolphin-emu-launch.gnplugin
python3 ../game-node-desktop/scripts/plugin-package.py verify dist/dolphin-emu-provider.gnplugin
python3 ../game-node-desktop/scripts/plugin-package.py verify dist/dolphin-emu-launch.gnplugin
python3 ../game-node-desktop/scripts/plugin-package.py bundle --component dist/dolphin-emu-provider.gnplugin --component dist/dolphin-emu-launch.gnplugin --id dev.richmont.gamenode.dolphin-emu.bundle --name 'Dolphin GameCube and Wii' --version 0.1.0 --private-key /tmp/dolphin-keys/catalog-dev.pem --output dist/dolphin-emu-0.1.0-linux-x64.gnbundle
python3 ../game-node-desktop/scripts/plugin-package.py bundle-verify dist/dolphin-emu-0.1.0-linux-x64.gnbundle
sha256sum dist/*.gnplugin dist/*.gnbundle
```

Import the bundle in GameNode, review the development publisher fingerprint, then grant each
component's permissions. The intended host flow provides one review, one inventory row and one
uninstall for both components. Package verification does not establish native UI acceptance.
The integration session must record those checks and a permitted consumer reading the durable
catalog after the provider stops. Launch remains a placeholder; known launch-session blockers
are outside this delivery. Windows cross-build and native acceptance remain separate work.

See [delivery evidence](docs/durable-catalog-evidence.md) for hashes and validation results.

## Documentation

- [Architecture](docs/architecture.md): why this is a provider plus an extension.
- [Protocol notes](docs/protocol-notes.md): exact RPCs, permissions and consent flow.
- [Tasks](TASKS.md): ordered work for the next contributor.
- [Contributing](CONTRIBUTING.md): conventions and validation.
- [OpenSpec](openspec/config.yaml): local specification workflow; active change artifacts live
  under `openspec/changes/`. CodeGraph's local index lives in the ignored `.codegraph/` directory.

## Layout

```text
provider/   GameNode provider: discovers Dolphin games
launch/     GameNode extension: runtime + custom-launch recipe
docs/       architecture and protocol notes
```

## License

GPL-3.0-only, like the rest of the GameNode project. Dolphin is a separate project with its own license; this plugin only reads its configuration
and starts an executable the user approves. It is not affiliated with the Dolphin project.
