# gamenode-plugin-dolphin-emu

A standalone GameNode plugin for **GameCube and Wii games managed by Dolphin**. The repository is
one workspace that builds two packages (discovery provider and launch extension); the intended
distribution is a single signed bundle, tracked as a GameNode host proposal. It is a separate
project on purpose: it exercises the public SDK, the package/consent conventions and the plugin
documentation as an external developer would see them.

## What works today

- **Discovery** (`provider/`): reads Dolphin's own `ISOPath0..N` entries from `Dolphin.ini`
  (Linux, Flatpak, Windows, macOS) plus `DOLPHIN_PLUGIN_ROOTS`, scans the folders recursively and
  reports `.iso`, `.gcm`, `.rvz`, `.wbfs`, `.wia`, `.ciso` and `.gcz` games. Plain discs expose
  their six-character game id (`GZLE01`, `RMCE01`); compressed containers fall back to a
  `GZLE01 - Title` file name or a stable path hash. Hidden folders, symlinks, deep trees and huge
  collections are bounded.
- **Launch recipe** (`launch/`): builds the Dolphin runtime (`-b -e <rom>`) and the immutable
  custom-launch recipe/content binding. The host supervises the process tree, so it detects when
  the game ends and attributes playtime; the plugin never starts a process itself.
- Tests: `cargo test` covers game-id parsing, compressed-container fallback, Dolphin config
  parsing, bounded scanning, runtime arguments and recipe immutability.

## What is still a task

`launch/` does not yet perform the session flow (find or register the Dolphin runtime, bind the
ROM handle, propose/request the approved recipe). `TASKS.md` lists it as the first task with the
exact SDK calls and permissions. The plugin intentionally does nothing unsafe while that is
pending.

## Build

The public SDK is not on crates.io yet. This workspace points at the reviewed sibling checkout
`../game-node-desktop/crates/plugin-sdk`; after the SDK is published, replace it with
`gamenode-plugin-sdk = "=0.1.0"` (see `Cargo.toml`).

```sh
CARGO_TARGET_DIR=/data/DADOS_LINUX/geral/area_troca_rust/target-dolphin-plugin \
  cargo test
CARGO_TARGET_DIR=/data/DADOS_LINUX/geral/area_troca_rust/target-dolphin-plugin \
  cargo build --release
```

## Install in GameNode

1. Build `gamenode-dolphin-provider` and `gamenode-dolphin-launch` for your platform.
2. Copy the provider binary to `provider/package/bin/linux-x64/plugin` (or the Windows
   `bin/windows-x64/plugin.exe`), the launch binary to the equivalent path in
   `launch/package/`.
3. Generate a development key, replace the placeholder publisher key in both manifests and pack
   each tree with `scripts/plugin-package.py` from the GameNode checkout:
   [`docs/plugin-package-manifest.md`](../game-node-desktop/docs/plugin-package-manifest.md#building-and-checking-a-package).
4. Import both packages, pin the publisher, install, review permissions and enable the provider.
   The provider requests `provider.context.read` (required) and `library.discovery`.

   Until GameNode ships bundle support (`openspec/changes/add-plugin-bundles`), two imports and
   two inventory rows are expected. The bundle will install both components from one signed file,
   show one review and one inventory row, and uninstall both together; the component packages
   remain the payload.
5. The library shows the Dolphin games. Launching them is the `launch/` task.

## Documentation

- [Architecture](docs/architecture.md): why this is a provider plus an extension.
- [Protocol notes](docs/protocol-notes.md): exact RPCs, permissions and consent flow.
- [Tasks](TASKS.md): ordered work for the next contributor.
- [Contributing](CONTRIBUTING.md): conventions and validation.

## Layout

```text
provider/   GameNode provider: discovers Dolphin games
launch/     GameNode extension: runtime + custom-launch recipe
docs/       architecture and protocol notes
```

## License

GPL-3.0-only, like the rest of the GameNode project. Dolphin is a separate project with its own license; this plugin only reads its configuration
and starts an executable the user approves. It is not affiliated with the Dolphin project.
