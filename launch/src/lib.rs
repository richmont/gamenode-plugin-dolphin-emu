//! GameNode extension that starts a GameCube/Wii game in Dolphin.
//!
//! The host owns process supervision: it launches the runtime, holds the resource leases and
//! reports the session as ended when the process tree exits. This extension only supplies the
//! approved runtime recipe.
//!
//! ## Session model
//!
//! GameNode starts an extension package as a one-shot, target-scoped process for **Start with
//! extension** (Play). The process receives the host-issued target and user intent through the
//! environment, opens an `Extension` session and performs the approval calls itself:
//!
//! 1. read the selected library entry (`LibraryGet`) and reject an unresolved catalog match;
//! 2. ask for the approved installation folder (`FilesRequestAccess`), list it and select the
//!    ROM file, then register a stable content binding (`GamesRegisterContent`);
//! 3. reuse an owned approved runtime or ask the host picker for one
//!    (`RuntimesRequestAccess`); the reply carries no executable path or handle;
//! 4. propose the immutable recipe once (`GamesProposeCustomLaunch`) and request the approved
//!    revision (`GamesRequestCustomLaunch`) with the host intent unchanged;
//! 5. persist only durable IDs and revisions in private Storage and revalidate them next time.
//!
//! `Initialize`/`Check` are readiness operations and never launch anything. The plugin never
//! starts a process and never invents a path, executable, argument or user intent.
//!
//! ## Host API limits observed by this implementation
//!
//! * `LibraryEntry` exposes the matched `game_id`, `installation_id`, name and revisions, but not
//!   the provider's discovery fields (`rom`, `platform`, `title`). The ROM is therefore selected
//!   from the host-approved installation listing by matching the entry title against the file
//!   name, or accepted only when the folder has exactly one Dolphin ROM. Ambiguity is refused.
//! * `FilesRequestAccess` and `RuntimesRequestAccess` wait for a user decision (up to five
//!   minutes on the host), while the SDK convenience helpers use a 15-second request deadline.
//!   Those two calls are built explicitly here with a five-minute deadline.
//! * A host launch needs the whole-library read grant to expose another provider's selected
//!   entry to an extension, so the manifest declares `library.read.all` in addition to
//!   `library.read`. The extension reads only the target installation.

use std::time::Duration;

use async_trait::async_trait;
use gamenode_plugin_sdk::{
    Durability, Error, ExtensionPlugin, HostClient, Notice, Resources, Severity, Status,
    capability, idempotency_key,
    proto::{
        self, CustomLaunchReply, FileEntry, FileHandle, JobState, LibraryEntry, MutationContext,
        MutationOutcome, OperationDisposition, OperationResult, ProviderCommand, ProviderOperation,
        RuntimeAccessHints, RuntimeMutationReply, RuntimeRecord, RuntimeRequestAccessRequest,
        ServiceContext, TargetGameContext,
    },
    resources::{
        CustomLaunchRecipe, RuntimeArgumentTemplate, RuntimeContentBinding, RuntimeExecutableHints,
        RuntimeSpec, Written,
    },
};
use serde::{Deserialize, Serialize};

pub const PLUGIN_ID: &str = "dev.richmont.gamenode.dolphin-emu-launch";
pub const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Content kind this runtime accepts. Dolphin loads both GameCube and Wii images; the host adds
/// the platform gate from `supported_platforms`, which is the host OS, not the console.
pub const CONTENT_KIND: &str = "rom";
/// Suffix-free content kinds this runtime accepts.
pub const SUPPORTED_FORMATS: &[&str] = &[CONTENT_KIND];
/// The runtime version is not readable without a path; the host picker owns the executable.
pub const RUNTIME_VERSION: &str = "unknown";
/// Extensions Dolphin can load. Kept in step with the provider's discovery list.
pub const ROM_EXTENSIONS: &[&str] = &[
    "iso", "gcm", "rvz", "wbfs", "wia", "ciso", "gcz", "dol", "elf",
];
/// Storage key prefix. The installation id follows when it fits the host's 256-byte key limit.
pub const STATE_PREFIX: &str = "launch.v1.";
/// The schema of the private launch record.
pub const STATE_SCHEMA: u32 = 1;
/// Interactive host approvals wait for the user for up to five minutes.
const APPROVAL_TIMEOUT: Duration = Duration::from_secs(5 * 60);
/// How many runtime pages to follow before declaring the catalog unusable.
const MAX_RUNTIME_PAGES: usize = 16;

pub const CAPABILITIES: &[&str] = &[
    capability::CONTEXT,
    capability::GAME_LAUNCH,
    capability::GAME_CUSTOM_LAUNCH,
    capability::GAME_RUNTIME_READ,
    capability::GAME_RUNTIME_CREATE,
    capability::FILES_READ,
    capability::LIBRARY_READ,
    capability::LIBRARY_READ_ALL,
    capability::DISCOVERY,
    capability::STORAGE_READ,
    capability::STORAGE_WRITE,
    capability::STORAGE_PERSIST,
    capability::NOTIFICATIONS_PUBLISH,
];

/// `dolphin-emu -b -e <rom>`: batch mode exits when the game ends, and the host replaces
/// `ContentPath` with the approved ROM path as one argv element.
pub fn dolphin_runtime(executable_handle: &str, version: &str) -> RuntimeSpec {
    RuntimeSpec {
        name: "Dolphin".into(),
        version: version.into(),
        executable_handle: executable_handle.into(),
        supported_formats: SUPPORTED_FORMATS
            .iter()
            .map(|format| (*format).to_owned())
            .collect(),
        extensions: Default::default(),
        platform: std::env::consts::OS.into(),
        argument_template: vec![
            RuntimeArgumentTemplate::Literal("-b".into()),
            RuntimeArgumentTemplate::Literal("-e".into()),
            RuntimeArgumentTemplate::ContentPath,
        ],
    }
}

/// The ROM handle is approved by the host for this game installation; the plugin never turns it
/// into a path.
pub fn content_binding(
    game_id: &str,
    installation_id: &str,
    content_handle: &str,
    runtime_id: &str,
) -> RuntimeContentBinding {
    RuntimeContentBinding {
        content_id: String::new(),
        game_id: game_id.into(),
        installation_id: installation_id.into(),
        content_handle: content_handle.into(),
        content_kind: CONTENT_KIND.into(),
        runtime_id: runtime_id.into(),
        // The host checks this against its own OS platform, not the console.
        supported_platforms: vec![std::env::consts::OS.into()],
    }
}

/// The approved recipe carries no argv or environment overrides: execution can only reuse the
/// immutable runtime and content the host validated.
pub fn recipe(
    game_id: &str,
    installation_id: &str,
    content_id: &str,
    runtime_id: &str,
    expected_revision: u64,
) -> CustomLaunchRecipe {
    CustomLaunchRecipe {
        game_id: game_id.into(),
        installation_id: installation_id.into(),
        content_id: content_id.into(),
        expected_revision,
        executable_handle: String::new(),
        uri: String::new(),
        argv: Vec::new(),
        working_directory_handle: String::new(),
        environment_set: Default::default(),
        environment_unset: Vec::new(),
        runtime_handle: runtime_id.into(),
        mod_layers: Vec::new(),
    }
}

/// The exact `FilesRequestAccess` request: request only read access to the installation folder,
/// declare no locations and let the user choose it in GameNode.
pub fn installation_access_request(
    context: &ServiceContext,
    game_id: &str,
    installation_id: &str,
    key: &str,
) -> proto::FilesRequestAccessRequest {
    proto::FilesRequestAccessRequest {
        mutation: Some(MutationContext {
            context: Some(context.clone()),
            operation_id: String::new(),
            idempotency_key: key.into(),
            expected_revision: 0,
        }),
        game_id: game_id.into(),
        installation_id: installation_id.into(),
        requested_operations: vec!["read".into()],
        resource_kinds: vec!["installation".into()],
        declared_locations: Vec::new(),
    }
}

fn proto_arguments(values: &[RuntimeArgumentTemplate]) -> Vec<proto::RuntimeArgumentTemplate> {
    values
        .iter()
        .map(|argument| proto::RuntimeArgumentTemplate {
            value: Some(match argument {
                RuntimeArgumentTemplate::Literal(value) => {
                    proto::runtime_argument_template::Value::Literal(value.clone())
                }
                RuntimeArgumentTemplate::ContentPath => {
                    proto::runtime_argument_template::Value::ContentPath(
                        proto::ContentPathArgument {},
                    )
                }
            }),
        })
        .collect()
}

fn runtime_record_from_spec(spec: &RuntimeSpec) -> RuntimeRecord {
    RuntimeRecord {
        id: String::new(),
        name: spec.name.clone(),
        version: spec.version.clone(),
        // The host chooses and validates the executable; a request must not carry a handle.
        executable_handle: String::new(),
        supported_formats: spec.supported_formats.clone(),
        extensions: spec.extensions.clone(),
        revision: 0,
        observed_at: 0,
        platform: spec.platform.clone(),
        argument_template: proto_arguments(&spec.argument_template),
        provenance: None,
    }
}

/// The exact `RuntimesRequestAccess` request: a runtime spec without an executable handle and
/// text hints only.
pub fn runtime_access_request(
    context: &ServiceContext,
    spec: &RuntimeSpec,
    hints: &RuntimeExecutableHints,
    key: &str,
) -> RuntimeRequestAccessRequest {
    RuntimeRequestAccessRequest {
        mutation: Some(MutationContext {
            context: Some(context.clone()),
            operation_id: String::new(),
            idempotency_key: key.into(),
            expected_revision: 0,
        }),
        runtime: Some(runtime_record_from_spec(spec)),
        hints: Some(RuntimeAccessHints {
            executable_names: hints.executable_names.clone(),
            desktop_ids: hints.desktop_ids.clone(),
        }),
    }
}

/// Hints for the host's runtime picker. Base names and desktop ids only, never a path.
pub fn runtime_hints() -> RuntimeExecutableHints {
    RuntimeExecutableHints {
        executable_names: vec!["dolphin-emu".into()],
        desktop_ids: vec!["dolphin-emu.desktop".into()],
    }
}

/// A refusal with text the user can act on. The code is stable for tests and diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchError {
    pub code: &'static str,
    pub message: String,
}

impl LaunchError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for LaunchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for LaunchError {}

/// The durable launch bookkeeping. It deliberately has no handles, paths, argv, environment or
/// user intent: those are session-scoped host values and must never be persisted.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LaunchState {
    pub schema: u32,
    pub installation_id: String,
    pub game_id: String,
    pub rom_name: String,
    /// Size and modification time of the selected ROM file. They detect an in-place replacement
    /// without storing the file's bytes; the host content approval itself has no digest.
    pub rom_size: u64,
    pub rom_modified_at: i64,
    pub runtime_id: String,
    pub runtime_revision: u64,
    pub content_id: String,
    pub content_revision: u64,
    pub recipe_id: String,
    pub recipe_revision: u64,
}

impl LaunchState {
    pub fn encode(&self) -> Result<Vec<u8>, LaunchError> {
        serde_json::to_vec(self)
            .map_err(|_| LaunchError::new("state_invalid", "The saved launch record is invalid."))
    }

    /// Reads a stored record. A missing, older-schema or malformed value is an empty record that
    /// may be replaced; a record for another installation is refused instead of overwritten.
    pub fn decode(
        stored: &gamenode_plugin_sdk::StoredValue,
        installation_id: &str,
    ) -> Result<Self, LaunchError> {
        if !stored.found {
            return Ok(Self::default());
        }
        let Ok(state) = serde_json::from_slice::<Self>(&stored.value) else {
            return Ok(Self::default());
        };
        if state.schema != STATE_SCHEMA {
            return Ok(Self::default());
        }
        if state.installation_id != installation_id {
            return Err(LaunchError::new(
                "state_owner_mismatch",
                "The saved launch record belongs to another installation. Reset this plugin's \
                 storage in GameNode and start the game again.",
            ));
        }
        Ok(state)
    }
}

/// The host-issued target and the resources needed by one launch session.
#[async_trait]
pub trait LaunchHost: Send + Sync {
    fn target(&self) -> &TargetGameContext;
    async fn library_get(&self, installation_id: &str) -> Result<LibraryEntry, Status>;
    async fn request_installation_access(
        &self,
        game_id: &str,
        installation_id: &str,
        key: &str,
    ) -> Result<FileHandle, Status>;
    async fn list_directory(&self, directory_handle: &str) -> Result<Vec<FileEntry>, Status>;
    async fn list_runtimes(&self) -> Result<Vec<RuntimeRecord>, Status>;
    async fn get_runtime(&self, runtime_id: &str) -> Result<RuntimeRecord, Status>;
    async fn request_runtime_access(
        &self,
        spec: &RuntimeSpec,
        hints: &RuntimeExecutableHints,
        key: &str,
    ) -> Result<RuntimeMutationReply, Status>;
    async fn register_content(
        &self,
        binding: &RuntimeContentBinding,
        expected_revision: u64,
        key: &str,
    ) -> Result<Written, Status>;
    async fn propose_recipe(
        &self,
        recipe: &CustomLaunchRecipe,
        intent: &str,
        key: &str,
    ) -> Result<CustomLaunchReply, Status>;
    #[allow(clippy::too_many_arguments)]
    async fn request_recipe(
        &self,
        game_id: &str,
        installation_id: &str,
        entry_revision: u64,
        recipe_id: &str,
        recipe_revision: u64,
        intent: &str,
        key: &str,
    ) -> Result<CustomLaunchReply, Status>;
    async fn storage_get(&self, key: &str) -> Result<gamenode_plugin_sdk::StoredValue, Status>;
    async fn storage_put(
        &self,
        key: &str,
        expected_revision: u64,
        key_idempotency: &str,
        value: Vec<u8>,
    ) -> Result<gamenode_plugin_sdk::StoredWrite, Status>;
    async fn notify_failure(&self, error: &LaunchError) -> Result<(), Status>;
}

/// Production host adapter. Interactive approvals use the raw resource client because the SDK
/// helpers cap every call at 15 seconds while the host waits up to five minutes for the user.
pub struct SdkHost {
    client: HostClient,
    resources: Resources,
    target: TargetGameContext,
}

impl SdkHost {
    pub fn new(host: &HostClient) -> Result<Self, LaunchError> {
        let target = host.context().target_game.clone().ok_or_else(|| {
            LaunchError::new(
                "no_target",
                "GameNode did not select a game for this extension session. Choose Play again from \
                 the game's page.",
            )
        })?;
        let resources = host.resources().ok_or_else(|| {
            LaunchError::new(
                "host_too_old",
                "This GameNode build does not expose the extension resource API. Update GameNode.",
            )
        })?;
        Ok(Self {
            client: host.clone(),
            resources,
            target,
        })
    }

    fn context(&self) -> Result<&ServiceContext, LaunchError> {
        self.client.resource_context().ok_or_else(|| {
            LaunchError::new(
                "host_too_old",
                "This GameNode build does not expose the extension resource API. Update GameNode.",
            )
        })
    }
}

#[async_trait]
impl LaunchHost for SdkHost {
    fn target(&self) -> &TargetGameContext {
        &self.target
    }

    async fn library_get(&self, installation_id: &str) -> Result<LibraryEntry, Status> {
        self.resources.library_get(installation_id).await
    }

    async fn request_installation_access(
        &self,
        game_id: &str,
        installation_id: &str,
        key: &str,
    ) -> Result<FileHandle, Status> {
        let context = self
            .context()
            .map_err(|error| Status::failed_precondition(error.message))?;
        let request = installation_access_request(context, game_id, installation_id, key);
        let mut request = tonic::Request::new(request);
        request.set_timeout(APPROVAL_TIMEOUT);
        let handles = self
            .client
            .resource_client()
            .files_request_access(request)
            .await?
            .into_inner()
            .handles;
        let mut handles = handles.into_iter();
        let handle = handles.next().ok_or_else(|| {
            Status::failed_precondition("GameNode returned no installation folder")
        })?;
        if handles.next().is_some() || handle.resource_kind != "installation" {
            return Err(Status::failed_precondition(
                "GameNode returned an unexpected installation folder answer",
            ));
        }
        Ok(handle)
    }

    async fn list_directory(&self, directory_handle: &str) -> Result<Vec<FileEntry>, Status> {
        self.resources.files_list(directory_handle).await
    }

    async fn list_runtimes(&self) -> Result<Vec<RuntimeRecord>, Status> {
        let mut runtimes = Vec::new();
        let mut cursor = String::new();
        for _ in 0..MAX_RUNTIME_PAGES {
            let page = self
                .resources
                .runtimes_page(gamenode_plugin_sdk::resources::MAX_PAGE_SIZE, &cursor, None)
                .await?;
            runtimes.extend(page.runtimes);
            if page.next_cursor.is_empty() {
                return Ok(runtimes);
            }
            cursor = page.next_cursor;
        }
        Err(Status::resource_exhausted(
            "the runtime catalog has too many pages",
        ))
    }

    async fn get_runtime(&self, runtime_id: &str) -> Result<RuntimeRecord, Status> {
        self.resources.runtime_get(runtime_id, None).await
    }

    async fn request_runtime_access(
        &self,
        spec: &RuntimeSpec,
        hints: &RuntimeExecutableHints,
        key: &str,
    ) -> Result<RuntimeMutationReply, Status> {
        let context = self
            .context()
            .map_err(|error| Status::failed_precondition(error.message))?;
        let request = runtime_access_request(context, spec, hints, key);
        let mut request = tonic::Request::new(request);
        request.set_timeout(APPROVAL_TIMEOUT);
        self.client
            .resource_client()
            .runtimes_request_access(request)
            .await
            .map(tonic::Response::into_inner)
    }

    async fn register_content(
        &self,
        binding: &RuntimeContentBinding,
        expected_revision: u64,
        key: &str,
    ) -> Result<Written, Status> {
        self.resources
            .register_runtime_content(binding.clone(), expected_revision, key)
            .await
    }

    async fn propose_recipe(
        &self,
        recipe: &CustomLaunchRecipe,
        intent: &str,
        key: &str,
    ) -> Result<CustomLaunchReply, Status> {
        self.resources
            .propose_custom_launch(recipe, intent, key)
            .await
    }

    async fn request_recipe(
        &self,
        game_id: &str,
        installation_id: &str,
        entry_revision: u64,
        recipe_id: &str,
        recipe_revision: u64,
        intent: &str,
        key: &str,
    ) -> Result<CustomLaunchReply, Status> {
        self.resources
            .request_custom_launch(
                game_id,
                installation_id,
                entry_revision,
                recipe_id,
                recipe_revision,
                intent,
                key,
            )
            .await
    }

    async fn storage_get(&self, key: &str) -> Result<gamenode_plugin_sdk::StoredValue, Status> {
        self.resources.storage_get(key).await
    }

    async fn storage_put(
        &self,
        key: &str,
        expected_revision: u64,
        key_idempotency: &str,
        value: Vec<u8>,
    ) -> Result<gamenode_plugin_sdk::StoredWrite, Status> {
        self.resources
            .storage_put(
                key,
                expected_revision,
                key_idempotency,
                value,
                Durability::Persistent,
                None,
            )
            .await
    }

    async fn notify_failure(&self, error: &LaunchError) -> Result<(), Status> {
        let notice = Notice::new("en", "Dolphin launch failed")
            .body(&error.message)
            .severity(Severity::Error);
        self.resources
            .notify(&notice, &idempotency_key())
            .await
            .map(|_| ())
    }
}

/// What play did, for tests and logging. The job is host-owned; no handle is in this report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchReport {
    pub game_id: String,
    pub installation_id: String,
    pub rom_name: String,
    pub runtime_id: String,
    pub runtime_revision: u64,
    pub content_id: String,
    pub content_revision: u64,
    pub recipe_id: String,
    pub recipe_revision: u64,
    pub job_id: String,
    pub state: i32,
    pub runtime_reused: bool,
    pub content_reused: bool,
    pub recipe_reused: bool,
}

struct Prepared {
    entry: LibraryEntry,
    rom_name: String,
    runtime_id: String,
    runtime_revision: u64,
    content_id: String,
    content_revision: u64,
    recipe_id: String,
    recipe_revision: u64,
    runtime_reused: bool,
    content_reused: bool,
    recipe_reused: bool,
}

#[derive(Default)]
pub struct DolphinLaunch;

impl DolphinLaunch {
    /// Runs one Play session. `intent` is the host-issued `GAMENODE_PLUGIN_USER_INTENT_ID`; it is
    /// passed through unchanged and never stored.
    pub async fn play<H: LaunchHost + ?Sized>(
        &self,
        host: &H,
        intent: Option<&str>,
    ) -> Result<LaunchReport, LaunchError> {
        let intent = intent.filter(|intent| !intent.is_empty()).ok_or_else(|| {
            LaunchError::new(
                "no_intent",
                "GameNode did not issue a launch intent. Start the game with this extension from \
                     GameNode's Play action.",
            )
        })?;
        let target = host.target();
        let game_id = target.game_id.trim().to_owned();
        let installation_id = target.installation_id.trim().to_owned();
        if game_id.is_empty() || installation_id.is_empty() {
            return Err(LaunchError::new(
                "no_target",
                "GameNode did not select a game for this extension session. Choose Play again from \
                 the game's page.",
            ));
        }

        let mut force_refresh = false;
        let mut last_refusal = None;
        for attempt in 0..2 {
            let prepared = self
                .prepare(host, &game_id, &installation_id, intent, force_refresh)
                .await?;
            let reply = host
                .request_recipe(
                    &prepared.entry.game_id,
                    &installation_id,
                    prepared.entry.revision,
                    &prepared.recipe_id,
                    prepared.recipe_revision,
                    intent,
                    &idempotency_key(),
                )
                .await;
            match reply {
                Ok(reply) => return finish(reply, &prepared),
                Err(status) if attempt == 0 && retryable_recipe_refusal(&status) => {
                    // The host refused before creating a job. A fresh recipe may repair a stale
                    // revision; the same intent is reused because nothing was spent.
                    last_refusal = Some(status);
                    force_refresh = true;
                }
                Err(status) => return Err(refusal("launch", status)),
            }
        }
        Err(refusal(
            "launch",
            last_refusal.unwrap_or_else(|| Status::aborted("the launch was refused")),
        ))
    }

    async fn prepare<H: LaunchHost + ?Sized>(
        &self,
        host: &H,
        game_id: &str,
        installation_id: &str,
        intent: &str,
        force_refresh: bool,
    ) -> Result<Prepared, LaunchError> {
        let entry = host
            .library_get(installation_id)
            .await
            .map_err(|status| refusal("library", status))?;
        // `LibraryEntry` exposes the matched identity and title, but not the provider's `rom`,
        // `platform` or `title` fields. The title is the only host-visible link to the file; the
        // approved installation listing below is the only source for the ROM handle.
        if entry.installation_id != installation_id {
            return Err(LaunchError::new(
                "unmatched",
                "GameNode returned a different installation for the selected game. Re-select the \
                 game and start it again.",
            ));
        }
        if entry.game_id.is_empty() || entry.game_id != game_id {
            return Err(LaunchError::new(
                "unmatched",
                "This game is not matched to a GameNode catalog entry, so an approved launch cannot \
                 be created. Match the game in GameNode's library (or fix the provider match) and \
                 start it again.",
            ));
        }
        let title = if entry.display_name.trim().is_empty() {
            entry.name.clone()
        } else {
            entry.display_name.clone()
        };
        if title.trim().is_empty() {
            return Err(LaunchError::new(
                "unmatched",
                "GameNode has no title for this game, so the ROM file cannot be identified. Restore \
                 the game's title or rescan it in the provider and start it again.",
            ));
        }

        let state_key = storage_key(installation_id);
        let stored = host
            .storage_get(&state_key)
            .await
            .map_err(|status| refusal("storage", status))?;
        let state = LaunchState::decode(&stored, installation_id)?;
        let state_revision = stored.revision;

        let directory = host
            .request_installation_access(game_id, installation_id, &idempotency_key())
            .await
            .map_err(|status| refusal("files", status))?;
        let files = host
            .list_directory(&directory.handle)
            .await
            .map_err(|status| refusal("files", status))?;
        let rom = select_rom(&entry.name, &entry.display_name, &files)?;

        let spec = dolphin_runtime("", RUNTIME_VERSION);
        let hints = runtime_hints();
        let (runtime, runtime_reused) =
            resolve_runtime(host, &state, &spec, &hints, force_refresh).await?;

        let content_changed = force_refresh
            || state.content_id.is_empty()
            || state.content_revision == 0
            || state.game_id != game_id
            || state.runtime_id != runtime.id
            || state.rom_name != rom.name
            || state.rom_size != rom.size_bytes
            || state.rom_modified_at != rom.modified_at;
        let (content_id, content_revision, content_reused) = if content_changed {
            let reuse_content_id = state.game_id == game_id && !state.content_id.is_empty();
            let mut binding = content_binding(game_id, installation_id, &rom.handle, &runtime.id);
            binding.content_id = if reuse_content_id {
                state.content_id.clone()
            } else {
                String::new()
            };
            let content_expected_revision = if reuse_content_id {
                state.content_revision
            } else {
                0
            };
            let written = host
                .register_content(&binding, content_expected_revision, &idempotency_key())
                .await
                .map_err(|status| refusal("content", status))?;
            let written = match written.outcome {
                gamenode_plugin_sdk::Outcome::Applied
                | gamenode_plugin_sdk::Outcome::AlreadyApplied => written,
                gamenode_plugin_sdk::Outcome::RevisionConflict => {
                    // The stored binding revision was stale. The reply carries the current one.
                    host.register_content(&binding, written.revision, &idempotency_key())
                        .await
                        .map_err(|status| refusal("content", status))?
                }
            };
            if !written.is_done() || written.resource_id.is_empty() || written.revision == 0 {
                return Err(LaunchError::new(
                    "content_refused",
                    "GameNode did not save the ROM content binding. Confirm the ROM file is still in \
                     the approved game folder and try again.",
                ));
            }
            (written.resource_id, written.revision, false)
        } else {
            (state.content_id.clone(), state.content_revision, true)
        };

        let recipe_changed = force_refresh
            || state.recipe_id.is_empty()
            || state.recipe_revision == 0
            || state.game_id != game_id
            || state.runtime_id != runtime.id
            || state.content_id != content_id
            || state.content_revision != content_revision;
        // Save the runtime/content approvals before proposing: a refused or interrupted proposal
        // must not force the user through the executable picker or folder approval again.
        let mut durable = LaunchState {
            schema: STATE_SCHEMA,
            installation_id: installation_id.into(),
            game_id: game_id.into(),
            rom_name: rom.name.clone(),
            rom_size: rom.size_bytes,
            rom_modified_at: rom.modified_at,
            runtime_id: runtime.id.clone(),
            runtime_revision: runtime.revision,
            content_id: content_id.clone(),
            content_revision,
            recipe_id: if recipe_changed {
                String::new()
            } else {
                state.recipe_id.clone()
            },
            recipe_revision: if recipe_changed {
                0
            } else {
                state.recipe_revision
            },
        };
        let mut durable_revision = state_revision;
        if durable != state || state_revision == 0 {
            durable_revision = persist_state(host, &state_key, &durable, state_revision).await?;
        }
        let interim = durable.clone();
        let (recipe_id, recipe_revision, recipe_reused) = if recipe_changed {
            let proposed = recipe(
                game_id,
                installation_id,
                &content_id,
                &runtime.id,
                entry.revision,
            );
            let reply = host
                .propose_recipe(&proposed, intent, &idempotency_key())
                .await
                .map_err(|status| refusal("recipe", status))?;
            if reply.recipe_id.is_empty() || reply.recipe_revision == 0 {
                return Err(LaunchError::new(
                    "recipe_refused",
                    "GameNode did not save the Dolphin launch recipe. Try starting the game again; \
                     if it repeats, remove and re-add the Dolphin extension.",
                ));
            }
            (reply.recipe_id, reply.recipe_revision, false)
        } else {
            (durable.recipe_id.clone(), durable.recipe_revision, true)
        };
        durable.recipe_id = recipe_id.clone();
        durable.recipe_revision = recipe_revision;
        if durable != interim {
            persist_state(host, &state_key, &durable, durable_revision).await?;
        }

        Ok(Prepared {
            entry,
            rom_name: rom.name,
            runtime_id: runtime.id,
            runtime_revision: runtime.revision,
            content_id,
            content_revision,
            recipe_id,
            recipe_revision,
            runtime_reused,
            content_reused,
            recipe_reused,
        })
    }
}

fn finish(reply: CustomLaunchReply, prepared: &Prepared) -> Result<LaunchReport, LaunchError> {
    let state = JobState::try_from(reply.state).unwrap_or(JobState::Unspecified);
    if !reply.refusal_code.is_empty() || state == JobState::Failed || state == JobState::Cancelled {
        let detail = if reply.message.trim().is_empty() {
            "GameNode refused the launch request."
        } else {
            reply.message.trim()
        };
        return Err(LaunchError::new(
            "launch_refused",
            format!(
                "{detail} Start the game again and review the Dolphin launch plan; if it repeats, \
                 remove the saved Dolphin runtime approval and approve the executable again."
            ),
        ));
    }
    if reply.job_id.is_empty() {
        return Err(LaunchError::new(
            "launch_refused",
            "GameNode did not create a launch job. Start the game again; if it repeats, remove and \
             re-add the Dolphin extension.",
        ));
    }
    Ok(LaunchReport {
        game_id: prepared.entry.game_id.clone(),
        installation_id: prepared.entry.installation_id.clone(),
        rom_name: prepared.rom_name.clone(),
        runtime_id: prepared.runtime_id.clone(),
        runtime_revision: prepared.runtime_revision,
        content_id: prepared.content_id.clone(),
        content_revision: prepared.content_revision,
        recipe_id: prepared.recipe_id.clone(),
        recipe_revision: prepared.recipe_revision,
        job_id: reply.job_id,
        state: reply.state,
        runtime_reused: prepared.runtime_reused,
        content_reused: prepared.content_reused,
        recipe_reused: prepared.recipe_reused,
    })
}

async fn resolve_runtime<H: LaunchHost + ?Sized>(
    host: &H,
    state: &LaunchState,
    spec: &RuntimeSpec,
    hints: &RuntimeExecutableHints,
    force_refresh: bool,
) -> Result<(RuntimeRecord, bool), LaunchError> {
    let mut reuse = None;
    if !force_refresh && !state.runtime_id.is_empty() {
        match host.get_runtime(&state.runtime_id).await {
            Ok(record)
                if runtime_matches(&record, spec)
                    && record.revision == state.runtime_revision
                    && record.revision > 0 =>
            {
                reuse = Some(record);
            }
            Ok(_) | Err(_) => {}
        }
        if reuse.is_none() {
            // The host may have hidden the record by id while a list still projects it.
            if let Ok(runtimes) = host.list_runtimes().await {
                if let Some(record) = runtimes.into_iter().find(|record| {
                    record.id == state.runtime_id
                        && runtime_matches(record, spec)
                        && record.revision == state.runtime_revision
                        && record.revision > 0
                }) {
                    reuse = Some(record);
                }
            }
        }
    }
    if let Some(record) = reuse {
        return Ok((record, true));
    }
    if !force_refresh && state.runtime_id.is_empty() {
        // No durable id yet: reuse an owned runtime with the same reviewed shape.
        if let Ok(runtimes) = host.list_runtimes().await {
            if let Some(record) = runtimes
                .into_iter()
                .find(|record| runtime_matches(record, spec) && record.revision > 0)
            {
                return Ok((record, true));
            }
        }
    }
    let reply = host
        .request_runtime_access(spec, hints, &idempotency_key())
        .await
        .map_err(|status| refusal("runtime", status))?;
    let outcome = MutationOutcome::try_from(reply.outcome).map_err(|_| {
        LaunchError::new(
            "runtime_refused",
            "GameNode returned an unknown runtime result.",
        )
    })?;
    let runtime = reply.runtime.ok_or_else(|| {
        if outcome == MutationOutcome::RevisionConflict {
            LaunchError::new(
                "runtime_changed",
                "The saved Dolphin runtime changed while it was being checked. Start the game again \
                 to review and approve it.",
            )
        } else {
            LaunchError::new(
                "runtime_refused",
                "GameNode did not approve a Dolphin runtime. Choose the Dolphin executable in the \
                 approval prompt, then start the game again.",
            )
        }
    })?;
    if runtime.id.is_empty() || runtime.revision == 0 {
        return Err(LaunchError::new(
            "runtime_refused",
            "GameNode returned an invalid Dolphin runtime. Start the game again; if it repeats, \
             remove and re-add the Dolphin extension.",
        ));
    }
    if !runtime_matches(&runtime, spec) {
        return Err(LaunchError::new(
            "runtime_changed",
            "The saved Dolphin runtime uses different launch arguments than this extension needs. \
             Remove the saved Dolphin runtime in GameNode's plugin settings, then start the game \
             again to approve it.",
        ));
    }
    Ok((runtime, false))
}

fn runtime_matches(record: &RuntimeRecord, spec: &RuntimeSpec) -> bool {
    let mut record_formats = record.supported_formats.clone();
    record_formats.sort();
    record_formats.dedup();
    let mut spec_formats = spec.supported_formats.clone();
    spec_formats.sort();
    spec_formats.dedup();
    record.name == spec.name
        && record.platform == spec.platform
        && record_formats == spec_formats
        && arguments_match(&record.argument_template, &spec.argument_template)
}

fn arguments_match(
    record: &[proto::RuntimeArgumentTemplate],
    spec: &[RuntimeArgumentTemplate],
) -> bool {
    record.len() == spec.len()
        && record
            .iter()
            .zip(spec)
            .all(|(record, spec)| match (record.value.as_ref(), spec) {
                (
                    Some(proto::runtime_argument_template::Value::Literal(record)),
                    RuntimeArgumentTemplate::Literal(spec),
                ) => record == spec,
                (
                    Some(proto::runtime_argument_template::Value::ContentPath(_)),
                    RuntimeArgumentTemplate::ContentPath,
                ) => true,
                _ => false,
            })
}

async fn persist_state<H: LaunchHost + ?Sized>(
    host: &H,
    key: &str,
    state: &LaunchState,
    expected_revision: u64,
) -> Result<u64, LaunchError> {
    let value = state.encode()?;
    let written = host
        .storage_put(key, expected_revision, &idempotency_key(), value.clone())
        .await
        .map_err(|status| refusal("storage", status))?;
    if matches!(
        written.outcome,
        gamenode_plugin_sdk::Outcome::Applied | gamenode_plugin_sdk::Outcome::AlreadyApplied
    ) {
        return Ok(written.revision);
    }
    // A concurrent session changed the key. Re-read it and replace it once; only launch when the
    // durable record now matches what this session is about to request.
    let current = host
        .storage_get(key)
        .await
        .map_err(|status| refusal("storage", status))?;
    let written = host
        .storage_put(key, current.revision, &idempotency_key(), value)
        .await
        .map_err(|status| refusal("storage", status))?;
    if matches!(
        written.outcome,
        gamenode_plugin_sdk::Outcome::Applied | gamenode_plugin_sdk::Outcome::AlreadyApplied
    ) {
        Ok(written.revision)
    } else {
        Err(LaunchError::new(
            "storage_conflict",
            "GameNode could not save the launch state because another session changed it. Close \
             other Play windows for this game and start it again.",
        ))
    }
}

/// The private Storage key for one installation. Long opaque ids are hashed to stay inside the
/// host's 256-byte key limit; the record itself carries the full id and is refused on mismatch.
pub fn storage_key(installation_id: &str) -> String {
    if STATE_PREFIX.len() + installation_id.len() <= 200 {
        format!("{STATE_PREFIX}{installation_id}")
    } else {
        format!(
            "{STATE_PREFIX}hash-{:016x}",
            fnv1a(installation_id.as_bytes())
        )
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// One host-listed file selected from the approved installation folder.
#[derive(Clone, Debug, PartialEq, Eq)]
struct SelectedRom {
    name: String,
    handle: String,
    size_bytes: u64,
    modified_at: i64,
}

/// Picks the ROM without ever seeing a path. The provider names an entry after its file, so the
/// entry title and file stem are compared after removing the optional six-character disc prefix.
/// A folder with exactly one Dolphin ROM is unambiguous even when the names differ; anything else
/// is refused rather than guessed.
fn select_rom(
    entry_name: &str,
    display_name: &str,
    files: &[FileEntry],
) -> Result<SelectedRom, LaunchError> {
    let candidates: Vec<&FileEntry> = files
        .iter()
        .filter(|file| !file.directory && !file.name.starts_with('.') && is_rom_name(&file.name))
        .collect();
    if candidates.is_empty() {
        return Err(LaunchError::new(
            "rom_not_found",
            "No GameCube/Wii ROM file was found in the approved game folder. Approve the folder \
             that contains the game and start it again.",
        ));
    }
    let wanted = [
        normalize_title(entry_name, false),
        normalize_title(display_name, false),
    ];
    let mut matches: Vec<&&FileEntry> = candidates
        .iter()
        .filter(|file| {
            let file_title = normalize_title(file_stem(&file.name), true);
            !file_title.is_empty() && wanted.contains(&file_title)
        })
        .collect();
    matches.dedup_by(|left, right| left.handle == right.handle);
    let chosen = match matches.len() {
        1 => matches.remove(0),
        0 if candidates.len() == 1 => &candidates[0],
        0 => {
            return Err(ambiguous_rom(&candidates));
        }
        _ => {
            return Err(ambiguous_rom(
                &matches.iter().map(|file| **file).collect::<Vec<_>>(),
            ));
        }
    };
    Ok(SelectedRom {
        name: chosen.name.clone(),
        handle: chosen.handle.clone(),
        size_bytes: chosen.size_bytes,
        modified_at: chosen.modified_at,
    })
}

fn ambiguous_rom(candidates: &[&FileEntry]) -> LaunchError {
    let mut names = candidates
        .iter()
        .take(5)
        .map(|file| file.name.as_str())
        .collect::<Vec<_>>();
    if candidates.len() > names.len() {
        names.push("…");
    }
    LaunchError::new(
        "rom_ambiguous",
        format!(
            "GameNode's approved folder contains several possible ROM files ({}), so the selected \
             game cannot be identified safely. Put this game in its own folder or make its file \
             name match the game title, then start it again.",
            names.join(", ")
        ),
    )
}

fn file_stem(name: &str) -> &str {
    name.rsplit_once('.')
        .map_or(name, |(stem, _extension)| stem)
}

fn is_rom_name(name: &str) -> bool {
    name.rsplit_once('.')
        .map(|(_stem, extension)| {
            ROM_EXTENSIONS
                .iter()
                .any(|known| extension.eq_ignore_ascii_case(known))
        })
        .unwrap_or(false)
}

/// Lower-cases and collapses separators. With `strip_disc_id`, a leading six-character disc
/// prefix such as `GZLE01 - ` is removed, exactly as the provider names its disc entries.
fn normalize_title(value: &str, strip_disc_id: bool) -> String {
    let mut value = value.trim();
    if strip_disc_id && value.len() >= 6 {
        let (prefix, rest) = value.split_at(6);
        let separated = rest.is_empty()
            || rest
                .chars()
                .next()
                .is_some_and(|character| matches!(character, ' ' | '-' | '_' | '.'));
        if separated
            && prefix
                .chars()
                .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit())
        {
            value = rest.trim_start_matches([' ', '-', '_', '.']);
        }
    }
    value
        .replace(['_', '.'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn refusal(step: &str, status: Status) -> LaunchError {
    let detail = status
        .message()
        .trim()
        .chars()
        .take(300)
        .collect::<String>();
    let (code, guidance) = match status.code() {
        gamenode_plugin_sdk::Code::PermissionDenied => (
            "permission_denied",
            match step {
                "library" => {
                    "GameNode did not allow reading the selected game's library entry. Grant the \
                     Dolphin launch extension library read (and whole-library read, used only for \
                     the selected game), then start the game again."
                }
                "files" => {
                    "GameNode did not allow access to the game's installation folder. Allow file \
                     access for this game and start it again."
                }
                "runtime" => {
                    "GameNode did not grant or approve a Dolphin runtime. Grant this extension the \
                     runtime create permission and choose Dolphin in the approval prompt, then start \
                     the game again."
                }
                "storage" => {
                    "GameNode did not allow persistent storage for this extension. Allow storage for \
                     the Dolphin extension and start the game again."
                }
                _ => {
                    "GameNode denied this launch permission. Review the Dolphin extension's \
                     permissions and start the game again."
                }
            },
        ),
        gamenode_plugin_sdk::Code::FailedPrecondition | gamenode_plugin_sdk::Code::Aborted => (
            "stale_state",
            match step {
                "library" => {
                    "The selected game changed in GameNode's library. Refresh the library and start \
                     it again."
                }
                "content" => {
                    "The saved ROM binding is stale or the file moved. Confirm the ROM is still in \
                     the approved folder and start the game again."
                }
                "recipe" => {
                    "The saved Dolphin launch plan changed. Start the game again to review the \
                     updated plan."
                }
                "runtime" => {
                    "The approved Dolphin executable changed or is missing. Approve the executable \
                     again, then start the game."
                }
                _ => {
                    "The launch approval is stale. Start the game again and review the updated plan."
                }
            },
        ),
        gamenode_plugin_sdk::Code::NotFound => (
            "missing_resource",
            match step {
                "library" => {
                    "GameNode could not see the selected game's library entry. Grant the Dolphin \
                     launch extension library read (and the whole-library read it needs for another \
                     provider's entry), then start the game again."
                }
                "files" => {
                    "GameNode no longer has the approved game folder. Approve the folder again and \
                     start the game."
                }
                "runtime" => {
                    "GameNode no longer has the approved Dolphin runtime. Approve the executable \
                     again and start the game."
                }
                "content" => {
                    "GameNode no longer has the saved ROM binding. Confirm the ROM file and start \
                     the game again."
                }
                "recipe" => {
                    "GameNode no longer has the saved Dolphin launch plan. Start the game again to \
                     review a new plan."
                }
                _ => {
                    "GameNode no longer has a required launch resource. Start the game again; if it \
                     repeats, remove and re-add the Dolphin extension."
                }
            },
        ),
        gamenode_plugin_sdk::Code::ResourceExhausted => (
            "quota",
            "GameNode is busy or a storage quota is full. Close other plugin windows, free space or \
             try again later.",
        ),
        gamenode_plugin_sdk::Code::Unauthenticated => (
            "session_expired",
            "The GameNode session expired. Start the game again from GameNode.",
        ),
        gamenode_plugin_sdk::Code::Unavailable | gamenode_plugin_sdk::Code::DeadlineExceeded => (
            "host_unavailable",
            "GameNode did not answer in time. Check the job list before starting the game again so \
             you do not start Dolphin twice.",
        ),
        _ => (
            "host_refused",
            "GameNode refused the launch request. Start the game again; if it repeats, remove and \
             re-add the Dolphin extension.",
        ),
    };
    let message = if detail.is_empty() {
        guidance.to_owned()
    } else {
        format!("{guidance} ({detail})")
    };
    LaunchError::new(code, message)
}

fn retryable_recipe_refusal(status: &Status) -> bool {
    matches!(
        status.code(),
        gamenode_plugin_sdk::Code::Aborted
            | gamenode_plugin_sdk::Code::NotFound
            | gamenode_plugin_sdk::Code::FailedPrecondition
    )
}

#[async_trait]
impl ExtensionPlugin for DolphinLaunch {
    /// Readiness only. GameNode starts an extension process for an action without sending it a
    /// command; the session work happens in [`DolphinLaunch::play`] from `main`. This handler
    /// therefore never launches and never asks for approvals.
    async fn handle(
        &self,
        command: ProviderCommand,
        _host: HostClient,
    ) -> Result<OperationResult, Error> {
        match ProviderOperation::try_from(command.operation) {
            Ok(ProviderOperation::Initialize)
            | Ok(ProviderOperation::Check)
            | Ok(ProviderOperation::Cancel)
            | Ok(ProviderOperation::Shutdown) => Ok(OperationResult {
                disposition: OperationDisposition::Completed as i32,
                ..Default::default()
            }),
            _ => Err(Error::Protocol),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, handle: &str) -> FileEntry {
        FileEntry {
            handle: handle.into(),
            name: name.into(),
            directory: false,
            size_bytes: 1,
            modified_at: 0,
        }
    }

    #[test]
    fn the_runtime_uses_batch_mode_and_a_single_content_argument() {
        let runtime = dolphin_runtime("handle-1", "2412");
        assert_eq!(runtime.name, "Dolphin");
        assert_eq!(runtime.executable_handle, "handle-1");
        assert_eq!(
            runtime.argument_template,
            [
                RuntimeArgumentTemplate::Literal("-b".into()),
                RuntimeArgumentTemplate::Literal("-e".into()),
                RuntimeArgumentTemplate::ContentPath,
            ]
        );
        assert_eq!(runtime.supported_formats, ["rom"]);
    }

    #[test]
    fn a_content_binding_keeps_the_host_identities_and_no_path() {
        let binding = content_binding("GZLE01", "id-1", "rom-handle", "runtime-1");
        assert_eq!(binding.game_id, "GZLE01");
        assert_eq!(binding.installation_id, "id-1");
        assert_eq!(binding.content_handle, "rom-handle");
        assert_eq!(binding.runtime_id, "runtime-1");
        assert_eq!(binding.content_kind, "rom");
        assert_eq!(binding.supported_platforms, [std::env::consts::OS]);
    }

    #[test]
    fn the_recipe_cannot_override_execution_details() {
        let recipe = recipe("GZLE01", "id-1", "content-1", "runtime-1", 7);
        assert_eq!(recipe.runtime_handle, "runtime-1");
        assert_eq!(recipe.content_id, "content-1");
        assert!(recipe.argv.is_empty());
        assert!(recipe.executable_handle.is_empty());
        assert!(recipe.working_directory_handle.is_empty());
        assert!(recipe.uri.is_empty());
        assert!(recipe.environment_set.is_empty() && recipe.environment_unset.is_empty());
    }

    #[test]
    fn the_installation_request_is_read_only_and_declares_no_location() {
        let context = ServiceContext {
            session_id: "session".into(),
            ..Default::default()
        };
        let request = installation_access_request(&context, "42", "install-1", "key-1");
        let mutation = request.mutation.unwrap();
        assert_eq!(request.game_id, "42");
        assert_eq!(request.installation_id, "install-1");
        assert_eq!(request.requested_operations, ["read"]);
        assert_eq!(request.resource_kinds, ["installation"]);
        assert!(request.declared_locations.is_empty());
        assert_eq!(mutation.expected_revision, 0);
        assert_eq!(mutation.idempotency_key, "key-1");
        assert_eq!(mutation.context.as_ref(), Some(&context));
    }

    #[test]
    fn the_runtime_request_has_no_executable_handle_and_only_text_hints() {
        let context = ServiceContext {
            session_id: "session".into(),
            ..Default::default()
        };
        let spec = dolphin_runtime("", RUNTIME_VERSION);
        let request = runtime_access_request(&context, &spec, &runtime_hints(), "key-1");
        let runtime = request.runtime.unwrap();
        let hints = request.hints.unwrap();
        assert!(runtime.executable_handle.is_empty());
        assert_eq!(runtime.name, "Dolphin");
        assert_eq!(runtime.platform, std::env::consts::OS);
        assert_eq!(runtime.argument_template.len(), 3);
        assert_eq!(hints.executable_names, ["dolphin-emu"]);
        assert_eq!(hints.desktop_ids, ["dolphin-emu.desktop"]);
    }

    #[test]
    fn rom_selection_uses_the_title_or_the_only_candidate() {
        let only = [file("GZLE01 - The Legend of Zelda.iso", "rom-1")];
        assert_eq!(
            select_rom("The Legend of Zelda", "", &only).unwrap(),
            SelectedRom {
                name: "GZLE01 - The Legend of Zelda.iso".into(),
                handle: "rom-1".into(),
                size_bytes: 1,
                modified_at: 0,
            }
        );
        let several = [
            file("Mario.iso", "rom-1"),
            file("Zelda.iso", "rom-2"),
            file("notes.txt", "notes"),
        ];
        assert_eq!(
            select_rom("Zelda", "", &several).unwrap(),
            SelectedRom {
                name: "Zelda.iso".into(),
                handle: "rom-2".into(),
                size_bytes: 1,
                modified_at: 0,
            }
        );
        assert_eq!(
            select_rom("Unknown", "", &several).unwrap_err().code,
            "rom_ambiguous"
        );
        assert_eq!(
            select_rom("Unknown", "", &[]).unwrap_err().code,
            "rom_not_found"
        );
        let duplicate = [file("Zelda.iso", "rom-1"), file("Zelda.rvz", "rom-2")];
        assert_eq!(
            select_rom("Zelda", "", &duplicate).unwrap_err().code,
            "rom_ambiguous"
        );
    }

    #[test]
    fn the_durable_record_contains_ids_and_revisions_but_no_session_values() {
        let state = LaunchState {
            schema: STATE_SCHEMA,
            installation_id: "install-1".into(),
            game_id: "42".into(),
            rom_name: "Zelda.iso".into(),
            rom_size: 1024,
            rom_modified_at: 1700000000,
            runtime_id: "runtime-1".into(),
            runtime_revision: 3,
            content_id: "content-1".into(),
            content_revision: 2,
            recipe_id: "recipe-1".into(),
            recipe_revision: 1,
        };
        let bytes = state.encode().unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(!text.contains("handle"));
        assert!(!text.contains("intent"));
        let stored = gamenode_plugin_sdk::StoredValue {
            found: true,
            value: bytes,
            revision: 4,
            durability: Some(Durability::Persistent),
            expires_at: 0,
        };
        assert_eq!(LaunchState::decode(&stored, "install-1").unwrap(), state);
        assert_eq!(
            LaunchState::decode(&stored, "install-2").unwrap_err().code,
            "state_owner_mismatch"
        );
    }

    #[test]
    fn storage_keys_fit_the_host_limit() {
        assert_eq!(storage_key("install-1"), "launch.v1.install-1");
        let long = "x".repeat(256);
        let key = storage_key(&long);
        assert!(key.len() <= 256);
        assert!(key.starts_with("launch.v1.hash-"));
        assert_eq!(key, storage_key(&long));
    }

    #[test]
    fn runtime_shape_compares_arguments_not_the_version() {
        let spec = dolphin_runtime("", "2412");
        let record = RuntimeRecord {
            id: "runtime-1".into(),
            name: "Dolphin".into(),
            version: "other".into(),
            executable_handle: String::new(),
            supported_formats: vec!["rom".into()],
            revision: 1,
            platform: std::env::consts::OS.into(),
            argument_template: proto_arguments(&spec.argument_template),
            ..Default::default()
        };
        assert!(runtime_matches(&record, &spec));
        let mut changed = record;
        changed.argument_template.pop();
        assert!(!runtime_matches(&changed, &spec));
    }

    #[test]
    fn the_manifest_declares_exactly_the_requested_capabilities() {
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../package/plugin.json")).unwrap();
        assert_eq!(manifest["id"], PLUGIN_ID);
        assert_eq!(manifest["version"], PLUGIN_VERSION);
        let permissions = manifest["permissions"].as_array().unwrap();
        let mut declared = std::collections::BTreeSet::new();
        for permission in permissions {
            let id = permission["id"].as_str().unwrap();
            assert!(declared.insert(id), "duplicate permission {id}");
            assert!(!permission["scope"].as_str().unwrap().trim().is_empty());
            assert!(!permission["reason"].as_str().unwrap().trim().is_empty());
            assert!(
                !permission["impact_if_disabled"]
                    .as_str()
                    .unwrap()
                    .trim()
                    .is_empty()
            );
        }
        let requested = CAPABILITIES
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(declared, requested);
    }
}
