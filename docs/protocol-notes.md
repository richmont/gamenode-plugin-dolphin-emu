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

Extensions use `Session::open_extension_from_launch(launch, plugin_id, version, capabilities)` and
`run_extension`; the launch environment carries the Core-selected target game.

`OpenSession` negotiates the highest overlap. The provider requests
`provider.context.read` and `library.discovery`; the extension requests `game.custom_launch` and
the runtime/file/library grants listed in its manifest. Permission review is separate: a declared
permission is not a grant.

## Discovery

`ProviderOperation::Discover` answers with `DiscoveryResult`:

```text
id          provider-local id (disc id or stable hash)
name        display title
install_dir folder containing the ROM (normalized, forward slashes)
fields      rom, platform, game_id
target      unset (no detected executable)
```

`provider.context.read` is checked first: the provider id in the reply must match `dolphin`.
Discovery is read-only and must not start a process.

The current `fields.game_id` identifies the disc; it must never populate an RPC's GameNode
`game_id`. The planned `disc_id` field will retain a compatibility reader for old entries.
Resolve the catalog ID and installation from the library/host-selected context. Use
`MatchingSearch` and `MatchingAssign` (`matching.read/write`) to establish catalog associations;
a private Storage entry is only a cache, not an authoritative match. Preserve user choices and
report unmatched or ambiguous games before attempting launch.

## Persistent plugin state (planned)

Use SDK `storage_get`, `storage_put`, `storage_delete` over `StorageGet/Put/Delete`.
Persistent writes require `storage.write` plus `storage.persist` and explicit
`Durability::Persistent`; reads and deletes require their respective permissions. `Persistence*`
endpoints administer usage/retention; key/value writes go through Storage.

Store versioned metadata/matching cache records in the provider namespace and durable
runtime/content/recipe references in the extension namespace. Namespace isolation includes plugin
ID, publisher fingerprint and active account: a common publisher or bundle does not share keys.
Read cross-component game state from the library API instead.

Respect the current 1 MiB value limit and 16 MiB quota per namespace/layer. Use bounded records,
idempotency keys and the last read storage revision; on conflict reread before retrying. Handle
cleanup, missing records and denied persistent writes without claiming durable success. Store no
session file handles, credentials or intent tokens. Storage revisions, library entry revisions,
content binding revisions and recipe revisions are separate values.

## Runtime registration

`Resources::runtime_register(RuntimeSpec { ... }, expected_revision, idempotency_key)`:

```text
name                "Dolphin"
version             detected Dolphin version
executable_handle   host-approved file handle (opaque)
supported_formats   gamecube-rom, wii-rom
platform            host OS
argument_template   Literal("-b"), Literal("-e"), ContentPath
```

The executable handle must be approved by the host. A plugin cannot turn an arbitrary path into a
handle; the executable must be inside an approved installation, prefix or library root. See
`TASKS.md` for the consent design.

`Resources::register_runtime_content(RuntimeContentBinding { ... }, expected_revision, key)` binds
one game installation to its ROM handle and runtime. The content handle is likewise host-issued.
The current host requires `library.discovery` and `files.read` for this call; the extension must
add discovery to its manifest and capabilities before implementing it. Revision 0 is for a new
binding; subsequent writes use the binding's own revision and stable content ID.

Resolve an owned, executable-approved runtime, not merely a row named `Dolphin`. Shared runtime
catalog visibility does not confer execution or mutation authority. A host-provided runtime is
an option only after its ownership/usage contract is demonstrated by an integration fixture.

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

The host shows the review and records the approved recipe revision. Later plays call
`request_custom_launch(game_id, installation_id, expected_revision, recipe_id, recipe_revision,
user_intent_id, key)`. Execution carries no replacement executable, argv, cwd, runtime or
environment payload, so the host can only run the immutable recipe it approved.

`user_intent_id` comes from `GAMENODE_PLUGIN_USER_INTENT_ID` (via `Launch::user_intent_id()`). It is
single-use, expires after 60 seconds and is bound to the extension, profile, game, installation,
operation and launch target.

Capture the intent before moving `Launch` into session setup. Initialization/check/readiness
alone must not trigger a launch. Verify first-use proposal, approval and execution against the
host; do not assume proposal executes the game or that a single-use intent can be consumed twice.
Later sessions recover durable IDs, reread current state and use the current action's intent.

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
