# Durable catalog and signed bundle: delivery evidence

Date: 2026-11-21. Branch: `feat/durable-catalog`. SDK/host reference:
`971a8a8acc1084e7a9c9e1163260ad6110df8b34` (`crates/plugin-sdk/src/resources.rs`).

This file records what was verified in this repository. It does not claim native GameNode
acceptance: the host integration session owns main-repo 6.3/7.1/7.2 and the Linux
import/review/inventory/uninstall checks.

## Contract decisions

| Item | Value |
| --- | --- |
| Dataset and schema | `dev.richmont.gamenode.dolphin-emu.catalog.v1` (schema version 1) |
| Durability | `PERSISTENT`, TTL 0 (host default) |
| Payload fields | `disc_id`, `title`, `platform`, `content_id` |
| Platform values | `gamecube`, `wii`, `unknown` |
| `resource_id` | disc ID when known, otherwise the scanner's stable opaque file ID |
| `content_id` | same value as `resource_id` |
| `expected_revision` | `0` (this API publishes append/replace observations, not entry edits) |
| Observation fields | fresh `source_observation_id` and idempotency key per scan, wall-clock `observed_at`, positive `sequence` |
| Mutation context | filled by the SDK helper; never built by the plugin |

Collapsing rule: copies, revisions and multidisc sets that share the six-character disc ID map to
one catalog row for that producer. The scanner's sorted order means the first record in a scan
wins. Rows owned by another publisher are separate, and a rescan replaces only the authenticated
producer's row by raising its host revision. Unknown-file IDs are path-derived hashes: they are
stable while the path is stable and are not content checksums. A rescan is not a deletion proof
for rows whose file disappeared; retention and uninstall policy stay with the host.

Excluded from the payload: filesystem paths, filenames, executable or content handles, argv,
working directory, environment, credentials and session handles. Provenance is host-assigned; the
plugin sends no owner/publisher fields. `opaque_payload` is never used.

Failure behavior: `host.resources().ok_or(Error::Protocol)?` on an older host; a denied
`data.publish` or `storage.persist` is reported as the explicit host error and never retried as
`SESSION_ONLY`; discovery records are still returned; publication stops at the first error in a
scan and nothing is logged beyond a status code and the SDK's safe error category.

## Permissions

`provider/package/plugin.json` adds `data.publish` and `storage.persist`, both `required: false`,
scope `own-provider`. ROM discovery does not need them, so a user who declines loses only the
durable catalog. `launch/package/plugin.json` is unchanged apart from the publisher key.

## Rust validation (2026-11-21)

All commands used `CARGO_TARGET_DIR=/data/DADOS_LINUX/geral/area_troca_rust/target-dolphin-plugin
CARGO_NET_OFFLINE=true CARGO_INCREMENTAL=0`, with no checkout-local `target/`.

```text
cargo fmt --all -- --check                       # clean
cargo clippy --workspace --all-targets -- -D warnings
                                                 # Finished, no warnings
cargo test --workspace                           # 11 passed, 0 failed
git diff --check                                 # clean
openspec validate --all --strict --no-interactive # 1 passed, 0 failed
```

`provider` unit tests: 7 passed. `provider/tests/catalog_reference_host.rs`: 4 passed, covering

- granted publish sends `DataPublish` with `PERSISTENT`, the expected dataset/schema/version, no
  opaque payload, and the session context supplied by the SDK helper;
- a rescan republishes the same `resource_id` with a new observation ID and raises the revision
  from 1 to 2;
- the durable row survives a reference-host restart and is readable through a fresh session;
- missing `storage.persist` is `Error::Denied`, creates no row, keeps discovery, and never
  downgrades the requested durability;
- missing `data.publish` is `Error::Denied`, creates no row, keeps discovery;
- unknown discs keep the opaque scanner ID and `platform = "unknown"`;
- the payload contains none of the ROM path, filename or credential bytes.

The reference host is an independent gRPC server built from public SDK messages and service
traits. Its on-disk state tests the client contract, not GameNode's production broker, and a real
consumer read with host approval is still an integration check.

## Signed artifacts

Release binaries were built with `cargo build --release --workspace`, copied to
`provider/package/bin/linux-x64/plugin` and `launch/package/bin/linux-x64/plugin`, and packaged
with `../game-node-desktop/scripts/plugin-package.py`. Notices were regenerated from
`cargo metadata --locked --format-version 1` with `scripts/dependency-notices.py`.

The development Ed25519 key lives outside the repository in `/tmp/dolphin-keys/catalog-dev.pem`;
binaries, packages, bundles and keys are gitignored and not committed. Both manifests carry the
same development publisher:

```text
key_id:     richmont-dolphin-emu-development
public_key: 9MJsveQ7SI656O7ykweL/Y+WN1pye9EJ9W+yaQZmidw=
```

Artifacts (outside Git, in `dist/`):

| Artifact | SHA-256 |
| --- | --- |
| `dist/dolphin-emu-provider.gnplugin` | `8d166aba41face67ff6c844d7d3b78571718118985a43d8051205bea526abc87` |
| `dist/dolphin-emu-launch.gnplugin` | `b9f956f4d728ecaf68c05764bad71bb693ede1a8263217d96734f90c5bfe536a` |
| `dist/dolphin-emu-0.3.2-x64.gnbundle` | `bcd3e603ff7e259e961cbc1f84f96cddb1d0617a490a2396bdc3ff328f19aee4` |

0.3.2 release (2026-11-22): the 0.3.1 development private key was lost with a reboot because it
was stored under `/tmp`, so this version is signed with a new development key kept at
`~/.config/gamenode/plugin-keys/dolphin-emu-dev.pem` (`0600`) and in Bitwarden. The publisher
`public_key` changed, which requires a fresh trust review before import; the old 0.3.1 packages
remain signed by the previous publisher. Both components now declare the
`windows/x86_64/msvc` target next to Linux, and the Windows binaries are static-CRT builds, so
they need no Visual C++ Redistributable. Native Windows acceptance is not recorded here.

Review contract (2026-11-21): both manifests now declare `supported_platforms`
`["gamecube", "wii"]` (content platforms, not OS targets) and a signed `impact_if_disabled` for
every permission, so the import review can explain what stops working when an optional grant is
switched off. The provider binary also carries the homebrew `.dol`/`.elf` discovery change, so the
hashes moved again; publisher key, plugin ids, provider id, bundle id/version and dataset
namespace are unchanged.

**Stale after 2026-11-22:** the launch session implementation changes
`launch/package/plugin.json` (adds `game.launch`, `library.read.all`, `library.discovery`,
storage and notification grants) and the launch binary. The package/bundle hashes below no longer
match the current source or manifest. Rebuild, re-sign and re-verify them before native acceptance;
see [launch-session-evidence.md](launch-session-evidence.md). The release tag and `dist/` artifacts
were not updated by that change.

Pre-release identity note (2026-11-21): the package, bundle, provider and dataset identifiers were
renamed from `org.example.gamenode.dolphin*` / `org.gamenode.dolphin.catalog.v1` to
`dev.richmont.gamenode.dolphin-emu*` / `dev.richmont.gamenode.dolphin-emu.catalog.v1`, and the
provider id is now `dolphin-emu`. The artifacts above were rebuilt and re-signed under the same
development public key; their hashes changed with the manifests. No released data, grants or
installed packages used the old identity.

Bundle: `dev.richmont.gamenode.dolphin-emu.bundle`, name `Dolphin GameCube and Wii`, version `0.3.2`.
`bundle.json` lists exactly two components, `dev.richmont.gamenode.dolphin-emu` (provider) and
`dev.richmont.gamenode.dolphin-emu-launch` (extension), with the component hashes above.

```text
plugin-package.py pack --source provider/package --private-key <key> --output dist/dolphin-emu-provider.gnplugin
plugin-package.py pack --source launch/package  --private-key <key> --output dist/dolphin-emu-launch.gnplugin
plugin-package.py verify dist/dolphin-emu-provider.gnplugin        # valid package
plugin-package.py verify dist/dolphin-emu-launch.gnplugin          # valid package
plugin-package.py bundle --component dist/dolphin-emu-provider.gnplugin \
  --component dist/dolphin-emu-launch.gnplugin --id dev.richmont.gamenode.dolphin-emu.bundle \
  --name 'Dolphin GameCube and Wii' --version 0.3.2 --private-key <key> \
  --output dist/dolphin-emu-0.3.2-x64.gnbundle                    # verified bundle
plugin-package.py bundle-verify dist/dolphin-emu-0.3.2-x64.gnbundle          # valid bundle
```

Exact reproducible commands are in `README.md`; the hashes change whenever a binary, manifest or
notice file changes, so re-record them after any repack.

## Remaining integration work

Main-repo `add-durable-shared-data` 6.3 and `add-plugin-bundles` 7.1/7.2 still need, on Linux:

- import the bundle and record one review, one inventory row and one uninstall;
- grant and revoke the catalog permissions and confirm a permitted consumer reads the durable
  record while the provider is stopped, including after a restart and after revocation;
- record host/plugin revisions and confirm retention and cleanup behavior;
- confirm the Windows cross-build and its native checks separately.

## Blockers

None for this delivery. The known launch-session blockers (library `fields` projection,
`CustomLaunch` intent mismatch, host approval route for an arbitrary emulator executable) remain
out of scope and were not worked around; they do not affect catalog publication or bundling.
