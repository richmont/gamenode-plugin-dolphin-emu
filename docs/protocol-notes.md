# Protocol notes

Everything here is the public API 1.6 surface served by GameNode Desktop. The SDK wrappers are in
`gamenode-plugin-sdk`; the canonical messages are in
`crates/plugin-protocol/proto/plugin.proto` of the GameNode checkout.

## Session

```rust
Session::open_with_options(
    Launch::from_env()?,
    PLUGIN_ID,
    PROVIDER_ID,
    PLUGIN_VERSION,
    CAPABILITIES,
    SessionOptions {
        min_api_major: 1,
        min_api_minor: 6,
        max_api_major: 1,
        max_api_minor: 6,
        features: vec![],
        session_kind: SessionKind::Provider,
        target_game: None,
    },
)
```

Extensions use `Session::open_extension_from_launch(launch, plugin_id, version, capabilities)`; the
launch environment carries the Core-selected target game and a `GAMENODE_PLUGIN_USER_INTENT_ID` for
a user-initiated action. Capture the intent with `Launch::user_intent_id()` before moving `Launch`
into session setup. A readiness startup has no intent: the extension opens and exits without any
resource call or launch. For an action, the extension performs the launch session itself and then
calls `run_extension` to keep the authenticated session alive until the host's launch job has
passed its approval fence and the host shuts the process down.

`OpenSession` negotiates the highest overlap. The provider requests
`provider.context.read` and `library.discovery`; the extension requests `game.launch`,
`game.custom_launch` and the runtime/file/library/storage/notification grants listed in its
manifest. Permission review is separate: a declared permission is not a grant.

## Discovery

`ProviderOperation::Discover` answers with `DiscoveryResult`:

```text
id          provider-local id (disc id, filename id, or stable path hash)
name        display title
install_dir folder containing the ROM (normalized, forward slashes)
fields      rom, platform, game_id
target      unset (no detected executable)
```

`provider.context.read` is checked first: the provider id in the reply must match `dolphin-emu`.
Discovery is read-only and must not start a process.

`.dol` and `.elf` homebrew executables are reported by the same scan. They have no disc header, so
they keep the path-derived `dolphin-emu-<hash>` id, send an empty `fields.game_id` and report
`platform = "unknown"`; the file name is not mined for a disc id. Launch is unchanged: the host
still passes the file to `-b -e`.

The current `fields.game_id` identifies the disc; it must never populate an RPC's GameNode
`game_id`. The planned `disc_id` field will retain a compatibility reader for old entries.
Resolve the catalog ID and installation from the library/host-selected context. Use
`MatchingSearch` and `MatchingAssign` (`matching.read/write`) to establish catalog associations;
a private Storage entry is only a cache, not an authoritative match. Preserve user choices and
report unmatched or ambiguous games before attempting launch.

## Persistent plugin state

Use SDK `storage_get`, `storage_put`, `storage_delete` over `StorageGet/Put/Delete`.
Persistent writes require `storage.write` plus `storage.persist` and explicit
`Durability::Persistent`; reads and deletes require their respective permissions. `Persistence*`
endpoints administer usage/retention; key/value writes go through Storage.

The provider namespace is reserved for versioned metadata/matching cache records (planned). The
extension namespace stores one `launch.v1.<installation>` record per installation with only
runtime/content/recipe IDs and revisions plus the selected ROM's name, size and modification time.
Namespace isolation includes plugin ID, publisher fingerprint and active account: a common
publisher or bundle does not share keys. Read cross-component game state from the library API
instead.

Respect the current 1 MiB value limit and 16 MiB quota per namespace/layer. Use bounded records,
idempotency keys and the last read storage revision; on conflict reread before retrying. Handle
cleanup, missing records and denied persistent writes without claiming durable success. Store no
session file handles, executable handles, paths, credentials or intent tokens. Storage revisions,
library entry revisions, content binding revisions and recipe revisions are separate values.

## Runtime registration

The extension asks the host to approve one exact executable; it never registers an arbitrary
handle and never receives a path or handle back:

```text
RuntimesRequestAccess
  runtime.name               "Dolphin"
  runtime.version            "unknown" (the plugin cannot read it without a path)
  runtime.executable_handle  empty; the host picker chooses and validates
  runtime.supported_formats  ["rom"]
  runtime.platform           host OS (linux/windows/macos)
  runtime.argument_template  Literal("-b"), Literal("-e"), ContentPath
  hints.executable_names     ["dolphin-emu"]
  hints.desktop_ids          ["dolphin-emu.desktop"]
```

The host reuses a valid owned runtime with the same name/platform/formats, otherwise shows its
picker. A declined or cancelled picker is `PERMISSION_DENIED` and creates nothing. The reply's
`executable_handle` is empty. `RuntimesList`/`RuntimesGet` are used to reuse a durable runtime;
a changed revision, shape or executable goes back through the request. The host revalidates the
executable identity and digest before every launch; a replaced executable requires reapproval.

`Resources::register_runtime_content(RuntimeContentBinding { ... }, expected_revision, key)` binds
one game installation to its ROM handle, a stable content ID and the runtime. The content handle is
host-issued for the approved installation folder. The host requires `library.discovery` and
`files.read` for this call. Revision 0 is for a new binding; a later move/rename uses the binding's
own revision and the same stable content ID. `content_kind` is `rom` and `supported_platforms` is
the host OS; the runtime accepts both GameCube and Wii images. Resolve an owned,
executable-approved runtime, not merely a row named `Dolphin`; shared runtime catalog visibility
does not confer execution or mutation authority.

## Custom launch

```rust
let recipe = CustomLaunchRecipe {
    game_id, installation_id, content_id, expected_revision,
    executable_handle: String::new(),
    uri: String::new(),
    argv: Vec::new(),
    working_directory_handle: String::new(),
    environment_set: BTreeMap::new(),
    environment_unset: Vec::new(),
    runtime_handle: runtime_id,
};
host.resources()?.propose_custom_launch(&recipe, user_intent_id, key).await?;
```

`expected_revision` is the current library entry revision. The host shows the review and records
the approved recipe revision. Later plays call
`request_custom_launch(game_id, installation_id, current_entry_revision, recipe_id,
recipe_revision, user_intent_id, key)`. Execution carries no replacement executable, argv, cwd,
runtime, content or environment payload, so the host can only run the immutable recipe it
approved. The request's `expected_revision` is the current entry revision, not the recipe revision.

`user_intent_id` comes from `GAMENODE_PLUGIN_USER_INTENT_ID` (via `Launch::user_intent_id()`). It is
single-use, expires after 60 seconds and is bound to the extension, profile, game, installation,
operation and launch target. Pass it unchanged; never persist or invent one. A custom-launch
request with the Play intent still goes through the host's recipe review/remembered approval
because the custom-launch binding includes the recipe and plan. Initialization/check/readiness
alone must not trigger a launch.

First use: read the entry, request the installation folder, select the ROM, reuse/request the
runtime, register the content binding, propose the recipe once, then request it. Later plays:
reuse valid runtime/content/recipe references and request only. A stale recipe, changed ROM,
runtime or content requires a fresh proposal and review. If the host refuses before creating a
job, the extension may repair a stale recipe once; after a job exists or on a transport timeout it
must not retry, to avoid starting Dolphin twice.

## Supervision and playtime

The host's launch supervisor:

- acquires the shared-resource leases atomically before spawning;
- tracks the process tree and holds the leases until a confirmed exit;
- reports the launch job state and attributes the session to playtime.

The plugin does not poll, signal or reap the emulator. `-b` is what makes Dolphin exit with the
game. Use `RuntimeSession`/`GamePresence` only for information the host cannot observe itself.

## References

- `docs/plugin-development.md` (GameNode checkout): provider lifecycle, SDK recipes.
- `docs/plugin-api-permissions.md`: method-to-permission matrix and consent.
- `docs/plugin-package-manifest.md`: manifest fields, signing and verification.
- `openspec/changes/third-party-plugin-custom-launch-runtimes/`: the design and 1.13 fixture for
  runtime recipes and supervised play.

## Durable shared catalog (implemented)

After worker-thread scanning, the provider asynchronously calls public SDK
`host.resources().ok_or(Error::Protocol)?` and `Resources::data_publish(key, 0, request)`.
The SDK supplies mutation/session context; the plugin never constructs it manually.
Dataset and schema are `dev.richmont.gamenode.dolphin-emu.catalog.v1`, version 1, `PERSISTENT`, TTL 0.
The string fields are `title`, `platform` and `content_id`, plus `disc_id` when a disc header or a
`GZLE01 - Title` file name supplied one. Homebrew executables publish no `disc_id` field at all
rather than an empty string. Platform is `gamecube`, `wii`, or `unknown` when the current scanner
cannot identify it. Title comes from the disc header (plain images and RVZ) when readable, else from the cleaned filename; WBFS/WIA still fall back to the file name. Dolphin's title database is not read yet.
There is no fabricated GameNode catalog ID: matching and local title database resolution remain
pending.

`resource_id` and `content_id` use the disc ID when known, otherwise the scanner's stable opaque
file ID. Copies, revisions and multiple discs sharing a six-character ID collapse into one
catalog row, distinct from installation identity. The first record in the scanner's sorted result
wins within a scan. Repeated scans replace only this authenticated producer's row and increase
its host revision; same-identity rows from another publisher remain separate. Unknown-file IDs
are path-derived hashes, not content checksums; moving such a file changes its identity. A rescan
is not a deletion or availability proof for older rows. Host retention/uninstall policy owns cleanup.

Only these metadata fields are published; discovery paths and filenames, handles, argv,
environment, credentials and session data are excluded. Provenance belongs to the host. Each
publication has a fresh observation/idempotency ID, observation time and positive sequence;
expected revision remains 0 under this API's append/replace contract. No automatic retry occurs.

Provider `data.publish` and `storage.persist` use `own-provider` scope and remain optional:
ROM discovery works without catalog publication. A host denial logs only its status code and SDK
safe category, returns discovery records, and never retries as SESSION_ONLY. Publication stops
at the first error in a scan. Persistent records are private by default and consumers still need
host sharing approval. Their lifetime is independent of the provider session, subject to retention
and revocation. This shared pool is separate from private Storage bookkeeping planned elsewhere.

SDK reference: GameNode integration `971a8a8` for the durable catalog; the launch session uses
`cc667cb` (approved runtime picker) and is recorded in
[launch-session-evidence.md](launch-session-evidence.md). See README for exact signed-bundle
commands and
[delivery evidence](durable-catalog-evidence.md). Production consumer/revocation/UI acceptance
belongs to the main-repo integration session, not the independent reference-host fixture.
