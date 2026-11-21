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
