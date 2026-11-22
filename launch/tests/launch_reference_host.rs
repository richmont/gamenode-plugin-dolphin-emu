//! Independent gRPC reference host for the Dolphin launch extension.
//!
//! It implements only the resource calls this extension is allowed to make and asserts the exact
//! wire shape: no executable handle, argv, working directory, environment or URI override is ever
//! accepted from the plugin, and the user intent is forwarded unchanged. The in-memory state is
//! the fixture's durable host store; it survives a new session, like a GameNode restart.
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use gamenode_dolphin_emu_launch::{
    CAPABILITIES, DolphinLaunch, LaunchState, PLUGIN_ID, STATE_PREFIX, SdkHost,
};
use gamenode_plugin_sdk::{ExtensionPlugin, Launch, Session, SessionOptions, proto::*};
use tokio_stream::wrappers::{ReceiverStream, TcpListenerStream};
use tonic::{Request, Response, Status};

mod support;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Call {
    LibraryGet(String),
    FilesAccess {
        game_id: String,
        installation_id: String,
        operations: Vec<String>,
        kinds: Vec<String>,
        declared: usize,
    },
    FilesList(String),
    StorageGet(String),
    StoragePut(String, u64),
    RuntimesList,
    RuntimesGet(String),
    RuntimeRequest {
        executable_handle: String,
        executable_names: Vec<String>,
        desktop_ids: Vec<String>,
    },
    ContentRegister {
        content_id: String,
        expected_revision: u64,
        content_handle: String,
        content_kind: String,
        runtime_id: String,
        supported_platforms: Vec<String>,
    },
    Propose {
        expected_revision: u64,
        content_id: String,
        runtime_handle: String,
        executable_handle: String,
        argv: Vec<String>,
        working_directory_handle: String,
        environment_set: BTreeMap<String, String>,
        environment_unset: Vec<String>,
        uri: String,
        mod_layers: usize,
        intent: String,
    },
    Request {
        expected_revision: u64,
        recipe_id: String,
        recipe_revision: u64,
        intent: String,
        executable_handle: String,
        argv: Vec<String>,
        working_directory_handle: String,
        environment_set: BTreeMap<String, String>,
        environment_unset: Vec<String>,
        uri: String,
        runtime_handle: String,
        content_id: String,
        mod_layers: usize,
    },
    Notify(String),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Counters {
    runtime_requests: usize,
    content_registers: usize,
    proposals: usize,
    launch_requests: usize,
    jobs: usize,
}

struct HostState {
    id: u64,
    target: TargetGameContext,
    entry: LibraryEntry,
    files: Vec<FileEntry>,
    runtimes: Vec<RuntimeRecord>,
    storage: HashMap<String, (Vec<u8>, u64)>,
    recipes: HashMap<String, (u64, CustomLaunchRequest)>,
    calls: Vec<Call>,
    runtime_requests: usize,
    content_registers: usize,
    proposals: usize,
    launch_requests: usize,
    jobs: usize,
    next_recipe: u64,
    deny_files: bool,
    deny_runtime: bool,
    deny_request: bool,
    refuse_request_once: bool,
}

#[derive(Clone)]
struct ReferenceHost {
    state: Arc<Mutex<HostState>>,
}

impl ReferenceHost {
    fn new(target: TargetGameContext, entry: LibraryEntry, files: Vec<FileEntry>) -> Self {
        Self {
            state: Arc::new(Mutex::new(HostState {
                id: 1,
                target,
                entry,
                files,
                runtimes: Vec::new(),
                storage: HashMap::new(),
                recipes: HashMap::new(),
                calls: Vec::new(),
                runtime_requests: 0,
                content_registers: 0,
                proposals: 0,
                launch_requests: 0,
                jobs: 0,
                next_recipe: 1,
                deny_files: false,
                deny_runtime: false,
                deny_request: false,
                refuse_request_once: false,
            })),
        }
    }

    fn calls(&self) -> Vec<Call> {
        self.state.lock().unwrap().calls.clone()
    }

    fn clear_calls(&self) {
        self.state.lock().unwrap().calls.clear();
    }

    fn counters(&self) -> Counters {
        let state = self.state.lock().unwrap();
        Counters {
            runtime_requests: state.runtime_requests,
            content_registers: state.content_registers,
            proposals: state.proposals,
            launch_requests: state.launch_requests,
            jobs: state.jobs,
        }
    }

    fn stored(&self, key: &str) -> (Vec<u8>, u64) {
        self.state
            .lock()
            .unwrap()
            .storage
            .get(key)
            .cloned()
            .unwrap_or_default()
    }

    fn authorize(&self, context: Option<&ServiceContext>) -> Result<(), Status> {
        let state = self.state.lock().unwrap();
        let context = context.ok_or_else(|| Status::unauthenticated("missing context"))?;
        if context.session_kind != SessionKind::Extension as i32
            || !context.provider_id.is_empty()
            || context.target_game.as_ref() != Some(&state.target)
            || context.session_credential != vec![9; 32]
        {
            return Err(Status::permission_denied("wrong extension scope"));
        }
        Ok(())
    }

    async fn open_session_impl(
        &self,
        request: Request<OpenSessionRequest>,
    ) -> Result<Response<OpenSessionReply>, Status> {
        let hello = request.into_inner();
        let (id, target) = {
            let mut state = self.state.lock().unwrap();
            let id = state.id;
            state.id += 1;
            (id, state.target.clone())
        };
        if hello.target_game.as_ref() != Some(&target)
            || hello.session_kind != SessionKind::Extension as i32
        {
            return Err(Status::permission_denied("wrong extension open"));
        }
        Ok(Response::new(OpenSessionReply {
            resource_context: Some(ServiceContext {
                session_id: format!("extension-{id}"),
                session_credential: hello.session_credential,
                provider_id: String::new(),
                operation_id: format!("operation-{id}"),
                account_epoch: 1,
                profile_epoch: 1,
                session_kind: SessionKind::Extension as i32,
                target_game: Some(target),
            }),
            session_id: format!("extension-{id}"),
            granted_capabilities: hello.requested_capabilities,
            api_major: 1,
            api_minor: 6,
            ..Default::default()
        }))
    }

    async fn exchange_impl(
        &self,
        _request: Request<tonic::Streaming<ProviderEvent>>,
    ) -> Result<Response<ReceiverStream<Result<ProviderCommand, Status>>>, Status> {
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn get_context_impl(
        &self,
        _request: Request<ServiceContext>,
    ) -> Result<Response<ProviderContext>, Status> {
        Ok(Response::new(ProviderContext {
            provider_id: String::new(),
            platform: std::env::consts::OS.into(),
            ..Default::default()
        }))
    }

    async fn library_get_impl(
        &self,
        request: Request<LibraryGetRequest>,
    ) -> Result<Response<LibraryGetReply>, Status> {
        let request = request.into_inner();
        self.authorize(request.context.as_ref())?;
        let mut state = self.state.lock().unwrap();
        state
            .calls
            .push(Call::LibraryGet(request.installation_id.clone()));
        Ok(Response::new(LibraryGetReply {
            entry: Some(state.entry.clone()),
            revision: state.entry.revision,
        }))
    }

    async fn files_request_access_impl(
        &self,
        request: Request<FilesRequestAccessRequest>,
    ) -> Result<Response<FileAccessReply>, Status> {
        let request = request.into_inner();
        let mutation = request
            .mutation
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("mutation"))?;
        self.authorize(mutation.context.as_ref())?;
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::FilesAccess {
            game_id: request.game_id.clone(),
            installation_id: request.installation_id.clone(),
            operations: request.requested_operations.clone(),
            kinds: request.resource_kinds.clone(),
            declared: request.declared_locations.len(),
        });
        if state.deny_files {
            return Err(Status::permission_denied("files.read"));
        }
        Ok(Response::new(FileAccessReply {
            handles: vec![FileHandle {
                handle: "directory-handle".into(),
                display_name: "The Legend of Zelda (installation)".into(),
                resource_kind: "installation".into(),
                writable: false,
            }],
        }))
    }

    async fn files_list_impl(
        &self,
        request: Request<FilesListRequest>,
    ) -> Result<Response<FilesListReply>, Status> {
        let request = request.into_inner();
        self.authorize(request.context.as_ref())?;
        let mut state = self.state.lock().unwrap();
        state
            .calls
            .push(Call::FilesList(request.directory_handle.clone()));
        Ok(Response::new(FilesListReply {
            entries: state.files.clone(),
            next_cursor: String::new(),
        }))
    }

    async fn runtimes_list_impl(
        &self,
        request: Request<DomainListRequest>,
    ) -> Result<Response<RuntimesListReply>, Status> {
        let request = request.into_inner();
        let page = request
            .page
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("page"))?;
        self.authorize(page.context.as_ref())?;
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::RuntimesList);
        Ok(Response::new(RuntimesListReply {
            runtimes: state.runtimes.clone(),
            next_cursor: String::new(),
            revision: state.runtimes.len() as u64,
        }))
    }

    async fn runtimes_get_impl(
        &self,
        request: Request<DomainGetRequest>,
    ) -> Result<Response<RuntimeRecordReply>, Status> {
        let request = request.into_inner();
        self.authorize(request.context.as_ref())?;
        let mut state = self.state.lock().unwrap();
        state
            .calls
            .push(Call::RuntimesGet(request.resource_id.clone()));
        let runtime = state
            .runtimes
            .iter()
            .find(|runtime| runtime.id == request.resource_id)
            .cloned()
            .ok_or_else(|| Status::not_found("runtime not found"))?;
        Ok(Response::new(RuntimeRecordReply {
            revision: runtime.revision,
            runtime: Some(runtime),
        }))
    }

    async fn runtimes_request_access_impl(
        &self,
        request: Request<RuntimeRequestAccessRequest>,
    ) -> Result<Response<RuntimeMutationReply>, Status> {
        let request = request.into_inner();
        let mutation = request
            .mutation
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("mutation"))?;
        self.authorize(mutation.context.as_ref())?;
        let runtime = request
            .runtime
            .ok_or_else(|| Status::invalid_argument("runtime"))?;
        let hints = request.hints.unwrap_or_default();
        let mut state = self.state.lock().unwrap();
        state.runtime_requests += 1;
        state.calls.push(Call::RuntimeRequest {
            executable_handle: runtime.executable_handle.clone(),
            executable_names: hints.executable_names.clone(),
            desktop_ids: hints.desktop_ids.clone(),
        });
        if state.deny_runtime {
            return Err(Status::permission_denied("the user declined the runtime"));
        }
        let existing = state
            .runtimes
            .iter()
            .find(|existing| {
                existing.name == runtime.name
                    && existing.platform == runtime.platform
                    && existing.supported_formats == runtime.supported_formats
            })
            .cloned();
        let runtime = if let Some(existing) = existing {
            // The picker revalidated the executable and returned the same owned runtime.
            existing
        } else {
            let record = RuntimeRecord {
                id: "runtime-1".into(),
                name: runtime.name,
                version: runtime.version,
                // The host picker never returns a handle, path or executable bytes.
                executable_handle: String::new(),
                supported_formats: runtime.supported_formats,
                revision: 1,
                platform: runtime.platform,
                argument_template: runtime.argument_template,
                ..Default::default()
            };
            state.runtimes.push(record.clone());
            record
        };
        Ok(Response::new(RuntimeMutationReply {
            runtime: Some(runtime),
            outcome: MutationOutcome::Applied as i32,
        }))
    }

    async fn games_register_content_impl(
        &self,
        request: Request<ContentRegistrationRequest>,
    ) -> Result<Response<MutationReply>, Status> {
        let request = request.into_inner();
        let mutation = request
            .mutation
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("mutation"))?;
        self.authorize(mutation.context.as_ref())?;
        let mut state = self.state.lock().unwrap();
        state.content_registers += 1;
        state.calls.push(Call::ContentRegister {
            content_id: request.content_id.clone(),
            expected_revision: mutation.expected_revision,
            content_handle: request.content_handle.clone(),
            content_kind: request.content_kind.clone(),
            runtime_id: request.runtime_id.clone(),
            supported_platforms: request.supported_platforms.clone(),
        });
        let runtime = state
            .runtimes
            .iter()
            .find(|runtime| runtime.id == request.runtime_id)
            .cloned()
            .ok_or_else(|| Status::not_found("runtime not found"))?;
        let content_id = if request.content_id.is_empty() {
            "content-1".to_owned()
        } else {
            request.content_id.clone()
        };
        let revision = mutation.expected_revision + 1;
        if revision == 0 {
            return Err(Status::invalid_argument("revision"));
        }
        let _ = runtime;
        Ok(Response::new(MutationReply {
            resource_id: content_id,
            revision,
            outcome: MutationOutcome::Applied as i32,
        }))
    }

    async fn games_propose_custom_launch_impl(
        &self,
        request: Request<CustomLaunchRequest>,
    ) -> Result<Response<CustomLaunchReply>, Status> {
        let request = request.into_inner();
        self.authorize(request.context.as_ref())?;
        let mut state = self.state.lock().unwrap();
        if request.expected_revision != state.entry.revision {
            return Err(Status::failed_precondition("library entry changed"));
        }
        if request.game_id != state.entry.game_id
            || request.installation_id != state.entry.installation_id
        {
            return Err(Status::not_found("library entry not found"));
        }
        state.proposals += 1;
        state.calls.push(Call::Propose {
            expected_revision: request.expected_revision,
            content_id: request.content_id.clone(),
            runtime_handle: request.runtime_handle.clone(),
            executable_handle: request.executable_handle.clone(),
            argv: request.argv.clone(),
            working_directory_handle: request.working_directory_handle.clone(),
            environment_set: request.environment_set.clone().into_iter().collect(),
            environment_unset: request.environment_unset.clone(),
            uri: request.uri.clone(),
            mod_layers: request.mod_layers.len(),
            intent: request.user_intent_id.clone(),
        });
        if !request.argv.is_empty()
            || !request.executable_handle.is_empty()
            || !request.working_directory_handle.is_empty()
            || !request.environment_set.is_empty()
            || !request.environment_unset.is_empty()
            || !request.uri.is_empty()
            || !request.mod_layers.is_empty()
            || !request.recipe_id.is_empty()
            || request.recipe_revision != 0
        {
            return Err(Status::invalid_argument(
                "the recipe must not carry execution overrides",
            ));
        }
        let recipe_id = format!("recipe-{}", state.next_recipe);
        state.next_recipe += 1;
        let revision = 1;
        state
            .recipes
            .insert(recipe_id.clone(), (revision, request.clone()));
        Ok(Response::new(CustomLaunchReply {
            recipe_id,
            recipe_revision: revision,
            job_id: String::new(),
            state: JobState::Unspecified as i32,
            message: "recipe saved".into(),
            ..Default::default()
        }))
    }

    async fn games_request_custom_launch_impl(
        &self,
        request: Request<CustomLaunchRequest>,
    ) -> Result<Response<CustomLaunchReply>, Status> {
        let request = request.into_inner();
        self.authorize(request.context.as_ref())?;
        let mut state = self.state.lock().unwrap();
        state.launch_requests += 1;
        state.calls.push(Call::Request {
            expected_revision: request.expected_revision,
            recipe_id: request.recipe_id.clone(),
            recipe_revision: request.recipe_revision,
            intent: request.user_intent_id.clone(),
            executable_handle: request.executable_handle.clone(),
            argv: request.argv.clone(),
            working_directory_handle: request.working_directory_handle.clone(),
            environment_set: request.environment_set.clone().into_iter().collect(),
            environment_unset: request.environment_unset.clone(),
            uri: request.uri.clone(),
            runtime_handle: request.runtime_handle.clone(),
            content_id: request.content_id.clone(),
            mod_layers: request.mod_layers.len(),
        });
        if !request.argv.is_empty()
            || !request.executable_handle.is_empty()
            || !request.working_directory_handle.is_empty()
            || !request.environment_set.is_empty()
            || !request.environment_unset.is_empty()
            || !request.uri.is_empty()
            || !request.runtime_handle.is_empty()
            || !request.content_id.is_empty()
            || !request.mod_layers.is_empty()
        {
            return Err(Status::invalid_argument(
                "execution must reference only the approved recipe",
            ));
        }
        if state.deny_request {
            return Err(Status::permission_denied("game.custom_launch"));
        }
        if state.refuse_request_once {
            state.refuse_request_once = false;
            return Err(Status::aborted("custom launch recipe revision changed"));
        }
        if request.expected_revision != state.entry.revision {
            return Err(Status::failed_precondition("library entry changed"));
        }
        let (revision, _stored) = state
            .recipes
            .get(&request.recipe_id)
            .cloned()
            .ok_or_else(|| Status::not_found("custom launch recipe not found"))?;
        if revision != request.recipe_revision {
            return Err(Status::aborted("custom launch recipe revision changed"));
        }
        state.jobs += 1;
        Ok(Response::new(CustomLaunchReply {
            recipe_id: request.recipe_id,
            recipe_revision: request.recipe_revision,
            job_id: format!("job-{}", state.jobs),
            state: JobState::Running as i32,
            message: "started".into(),
            ..Default::default()
        }))
    }

    async fn storage_get_impl(
        &self,
        request: Request<StorageGetRequest>,
    ) -> Result<Response<StorageGetReply>, Status> {
        let request = request.into_inner();
        self.authorize(request.context.as_ref())?;
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::StorageGet(request.key.clone()));
        let (value, revision) = state.storage.get(&request.key).cloned().unwrap_or_default();
        Ok(Response::new(StorageGetReply {
            value,
            revision,
            found: revision > 0,
            durability: StorageDurability::Persistent as i32,
            expires_at: 0,
        }))
    }

    async fn storage_put_impl(
        &self,
        request: Request<StoragePutRequest>,
    ) -> Result<Response<StorageMutationReply>, Status> {
        let request = request.into_inner();
        let mutation = request
            .mutation
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("mutation"))?;
        self.authorize(mutation.context.as_ref())?;
        if request.durability != StorageDurability::Persistent as i32 {
            return Err(Status::permission_denied("storage.persist"));
        }
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::StoragePut(
            request.key.clone(),
            mutation.expected_revision,
        ));
        let current = state
            .storage
            .get(&request.key)
            .map(|(_, revision)| *revision);
        if current.unwrap_or(0) != mutation.expected_revision {
            return Ok(Response::new(StorageMutationReply {
                revision: current.unwrap_or(0),
                outcome: MutationOutcome::RevisionConflict as i32,
                durability: StorageDurability::Persistent as i32,
                ..Default::default()
            }));
        }
        let revision = current.unwrap_or(0) + 1;
        state
            .storage
            .insert(request.key.clone(), (request.value.clone(), revision));
        Ok(Response::new(StorageMutationReply {
            revision,
            usage_bytes: request.value.len() as u64,
            quota_bytes: 16 * 1024 * 1024,
            outcome: MutationOutcome::Applied as i32,
            durability: StorageDurability::Persistent as i32,
            expires_at: 0,
        }))
    }

    async fn notifications_publish_impl(
        &self,
        request: Request<NotificationPublishRequest>,
    ) -> Result<Response<MutationReply>, Status> {
        let request = request.into_inner();
        let mutation = request
            .mutation
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("mutation"))?;
        self.authorize(mutation.context.as_ref())?;
        let mut state = self.state.lock().unwrap();
        state
            .calls
            .push(Call::Notify(format!("{}: {}", request.title, request.body)));
        Ok(Response::new(MutationReply {
            resource_id: "notice-1".into(),
            revision: 1,
            outcome: MutationOutcome::Applied as i32,
        }))
    }
}

async fn connect(
    reference: ReferenceHost,
    target: TargetGameContext,
) -> (
    Session,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let host = reference.clone();
    let task = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(plugin_host_server::PluginHostServer::new(host.clone()))
            .add_service(provider_host_server::ProviderHostServer::new(host.clone()))
            .add_service(resource_host_server::ResourceHostServer::new(host))
            .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    let channel = tonic::transport::Endpoint::from_shared(format!("http://{address}"))
        .unwrap()
        .connect()
        .await
        .unwrap();
    let session = Session::from_channel_with_options(
        channel,
        Launch {
            endpoint: PathBuf::new(),
            credential: vec![9; 32],
            operation_id: "extension-launch".into(),
        },
        PLUGIN_ID,
        "",
        "0.2.0",
        CAPABILITIES,
        SessionOptions {
            min_api_major: 1,
            min_api_minor: 6,
            max_api_major: 1,
            max_api_minor: 6,
            session_kind: SessionKind::Extension,
            target_game: Some(target),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    (session, stop, task)
}

fn target() -> TargetGameContext {
    TargetGameContext {
        game_id: "42".into(),
        installation_id: "install-1".into(),
        profile_id: "account-a".into(),
        revision: "100".into(),
    }
}

fn entry() -> LibraryEntry {
    LibraryEntry {
        game_id: "42".into(),
        installation_id: "install-1".into(),
        provider_id: "dolphin-emu".into(),
        name: "The Legend of Zelda".into(),
        install_state: "installed".into(),
        revision: 7,
        ..Default::default()
    }
}

fn files() -> Vec<FileEntry> {
    vec![
        FileEntry {
            handle: "rom-handle-1".into(),
            name: "GZLE01 - The Legend of Zelda.iso".into(),
            size_bytes: 1024,
            modified_at: 1,
            directory: false,
        },
        FileEntry {
            handle: "notes-handle".into(),
            name: "notes.txt".into(),
            size_bytes: 10,
            modified_at: 1,
            directory: false,
        },
    ]
}

fn fixture() -> ReferenceHost {
    ReferenceHost::new(target(), entry(), files())
}

async fn start(
    reference: &ReferenceHost,
) -> (
    Session,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    connect(reference.clone(), target()).await
}

async fn stop(
    session: Session,
    shutdown: tokio::sync::oneshot::Sender<()>,
    task: tokio::task::JoinHandle<()>,
) {
    drop(session);
    let _ = shutdown.send(());
    let _ = task.await;
}

#[tokio::test]
async fn first_play_uses_only_opaque_host_values_and_never_execution_overrides() {
    let reference = fixture();
    let (session, shutdown, task) = start(&reference).await;
    let host = SdkHost::new(&session.host).unwrap();
    let report = DolphinLaunch
        .play(&host, Some("intent-first"))
        .await
        .unwrap();
    assert_eq!(report.job_id, "job-1");
    assert_eq!(report.state, JobState::Running as i32);
    assert_eq!(report.rom_name, "GZLE01 - The Legend of Zelda.iso");
    assert_eq!(report.runtime_id, "runtime-1");
    assert_eq!(report.content_id, "content-1");
    assert_eq!(report.recipe_id, "recipe-1");
    assert!(!report.runtime_reused);
    assert!(!report.content_reused);
    assert!(!report.recipe_reused);

    let calls = reference.calls();
    let expected = vec![
        Call::LibraryGet("install-1".into()),
        Call::StorageGet(format!("{STATE_PREFIX}install-1")),
        Call::FilesAccess {
            game_id: "42".into(),
            installation_id: "install-1".into(),
            operations: vec!["read".into()],
            kinds: vec!["installation".into()],
            declared: 0,
        },
        Call::FilesList("directory-handle".into()),
        Call::RuntimesList,
        Call::RuntimeRequest {
            executable_handle: String::new(),
            executable_names: vec!["dolphin-emu".into()],
            desktop_ids: vec!["dolphin-emu.desktop".into()],
        },
        Call::ContentRegister {
            content_id: String::new(),
            expected_revision: 0,
            content_handle: "rom-handle-1".into(),
            content_kind: "rom".into(),
            runtime_id: "runtime-1".into(),
            supported_platforms: vec![std::env::consts::OS.into()],
        },
        Call::StoragePut(format!("{STATE_PREFIX}install-1"), 0),
        Call::Propose {
            expected_revision: 7,
            content_id: "content-1".into(),
            runtime_handle: "runtime-1".into(),
            executable_handle: String::new(),
            argv: Vec::new(),
            working_directory_handle: String::new(),
            environment_set: BTreeMap::new(),
            environment_unset: Vec::new(),
            uri: String::new(),
            mod_layers: 0,
            intent: "intent-first".into(),
        },
        Call::StoragePut(format!("{STATE_PREFIX}install-1"), 1),
        Call::Request {
            expected_revision: 7,
            recipe_id: "recipe-1".into(),
            recipe_revision: 1,
            intent: "intent-first".into(),
            executable_handle: String::new(),
            argv: Vec::new(),
            working_directory_handle: String::new(),
            environment_set: BTreeMap::new(),
            environment_unset: Vec::new(),
            uri: String::new(),
            runtime_handle: String::new(),
            content_id: String::new(),
            mod_layers: 0,
        },
    ];
    assert_eq!(calls, expected);

    let (bytes, revision) = reference.stored(&format!("{STATE_PREFIX}install-1"));
    assert_eq!(revision, 2);
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(!text.contains("rom-handle"));
    assert!(!text.contains("intent-first"));
    assert!(!text.contains("directory-handle"));
    let saved: LaunchState = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(saved.runtime_id, "runtime-1");
    assert_eq!(saved.content_id, "content-1");
    assert_eq!(saved.recipe_id, "recipe-1");
    assert_eq!(saved.recipe_revision, 1);
    assert_eq!(reference.counters().jobs, 1);

    stop(session, shutdown, task).await;
}

#[tokio::test]
async fn repeated_play_reuses_the_runtime_content_and_recipe() {
    let reference = fixture();
    let (session, shutdown, task) = start(&reference).await;
    let host = SdkHost::new(&session.host).unwrap();
    DolphinLaunch
        .play(&host, Some("intent-first"))
        .await
        .unwrap();
    stop(session, shutdown, task).await;

    reference.clear_calls();
    let (session, shutdown, task) = start(&reference).await;
    let host = SdkHost::new(&session.host).unwrap();
    let report = DolphinLaunch
        .play(&host, Some("intent-second"))
        .await
        .unwrap();
    assert_eq!(report.recipe_id, "recipe-1");
    assert_eq!(report.recipe_revision, 1);
    assert!(report.runtime_reused);
    assert!(report.content_reused);
    assert!(report.recipe_reused);
    assert_eq!(report.job_id, "job-2");

    let calls = reference.calls();
    assert!(
        calls
            .iter()
            .all(|call| !matches!(call, Call::RuntimeRequest { .. })),
        "a valid runtime must not prompt again: {calls:?}"
    );
    assert!(
        calls
            .iter()
            .all(|call| !matches!(call, Call::ContentRegister { .. })),
        "an unchanged binding must not bump its revision: {calls:?}"
    );
    assert!(
        calls
            .iter()
            .all(|call| !matches!(call, Call::Propose { .. })),
        "an approved recipe must not be proposed again: {calls:?}"
    );
    assert!(calls.contains(&Call::RuntimesGet("runtime-1".into())));
    assert!(calls.contains(&Call::Request {
        expected_revision: 7,
        recipe_id: "recipe-1".into(),
        recipe_revision: 1,
        intent: "intent-second".into(),
        executable_handle: String::new(),
        argv: Vec::new(),
        working_directory_handle: String::new(),
        environment_set: BTreeMap::new(),
        environment_unset: Vec::new(),
        uri: String::new(),
        runtime_handle: String::new(),
        content_id: String::new(),
        mod_layers: 0,
    }));
    let counters = reference.counters();
    assert_eq!(counters.runtime_requests, 1);
    assert_eq!(counters.content_registers, 1);
    assert_eq!(counters.proposals, 1);
    assert_eq!(counters.jobs, 2);

    stop(session, shutdown, task).await;
}

#[tokio::test]
async fn a_changed_runtime_requires_fresh_host_approval() {
    let reference = fixture();
    let (session, shutdown, task) = start(&reference).await;
    let host = SdkHost::new(&session.host).unwrap();
    DolphinLaunch
        .play(&host, Some("intent-first"))
        .await
        .unwrap();
    stop(session, shutdown, task).await;

    {
        let mut state = reference.state.lock().unwrap();
        state.runtimes[0].revision = 2;
        state.runtimes[0].version = "2412".into();
        state.calls.clear();
    }
    let (session, shutdown, task) = start(&reference).await;
    let host = SdkHost::new(&session.host).unwrap();
    let report = DolphinLaunch
        .play(&host, Some("intent-second"))
        .await
        .unwrap();
    assert_eq!(report.runtime_revision, 2);
    assert_eq!(report.recipe_id, "recipe-1");
    let calls = reference.calls();
    assert!(
        calls
            .iter()
            .any(|call| matches!(call, Call::RuntimeRequest { executable_handle, .. } if executable_handle.is_empty())),
        "the changed runtime must go through the host picker again: {calls:?}"
    );
    let counters = reference.counters();
    assert_eq!(counters.runtime_requests, 2);
    assert_eq!(counters.proposals, 1);
    assert_eq!(counters.jobs, 2);

    stop(session, shutdown, task).await;
}

#[tokio::test]
async fn a_changed_content_requires_a_new_binding_and_recipe_review() {
    let reference = fixture();
    let (session, shutdown, task) = start(&reference).await;
    let host = SdkHost::new(&session.host).unwrap();
    DolphinLaunch
        .play(&host, Some("intent-first"))
        .await
        .unwrap();
    stop(session, shutdown, task).await;

    {
        let mut state = reference.state.lock().unwrap();
        state.files = vec![FileEntry {
            handle: "rom-handle-2".into(),
            name: "GZLE01 - The Legend of Zelda.rvz".into(),
            size_bytes: 2048,
            modified_at: 2,
            directory: false,
        }];
        state.calls.clear();
    }
    let (session, shutdown, task) = start(&reference).await;
    let host = SdkHost::new(&session.host).unwrap();
    let report = DolphinLaunch
        .play(&host, Some("intent-second"))
        .await
        .unwrap();
    assert_eq!(report.rom_name, "GZLE01 - The Legend of Zelda.rvz");
    assert_eq!(report.content_id, "content-1");
    assert_eq!(report.content_revision, 2);
    assert_eq!(report.recipe_id, "recipe-2");
    assert!(!report.content_reused);
    assert!(!report.recipe_reused);
    let calls = reference.calls();
    assert!(calls.iter().any(|call| matches!(
        call,
        Call::ContentRegister {
            content_id,
            expected_revision: 1,
            content_handle,
            ..
        } if content_id == "content-1" && content_handle == "rom-handle-2"
    )));
    assert!(
        calls
            .iter()
            .any(|call| matches!(call, Call::Propose { .. }))
    );
    let counters = reference.counters();
    assert_eq!(counters.content_registers, 2);
    assert_eq!(counters.proposals, 2);
    assert_eq!(counters.jobs, 2);

    stop(session, shutdown, task).await;
}

#[tokio::test]
async fn a_changed_rom_at_the_same_path_requires_a_new_binding_and_recipe_review() {
    let reference = fixture();
    let (session, shutdown, task) = start(&reference).await;
    let host = SdkHost::new(&session.host).unwrap();
    DolphinLaunch
        .play(&host, Some("intent-first"))
        .await
        .unwrap();
    stop(session, shutdown, task).await;

    {
        let mut state = reference.state.lock().unwrap();
        // Same file name and handle, replaced content: size/mtime changed.
        state.files[0].size_bytes = 4096;
        state.files[0].modified_at = 99;
        state.calls.clear();
    }
    let (session, shutdown, task) = start(&reference).await;
    let host = SdkHost::new(&session.host).unwrap();
    let report = DolphinLaunch
        .play(&host, Some("intent-second"))
        .await
        .unwrap();
    assert_eq!(report.content_revision, 2);
    assert_eq!(report.recipe_id, "recipe-2");
    let calls = reference.calls();
    assert!(calls.iter().any(|call| matches!(
        call,
        Call::ContentRegister {
            expected_revision: 1,
            ..
        }
    )));
    assert!(
        calls
            .iter()
            .any(|call| matches!(call, Call::Propose { .. }))
    );
    let counters = reference.counters();
    assert_eq!(counters.content_registers, 2);
    assert_eq!(counters.proposals, 2);
    stop(session, shutdown, task).await;
}

#[tokio::test]
async fn a_stale_recipe_is_replaced_once_without_a_second_job() {
    let reference = fixture();
    {
        let mut state = reference.state.lock().unwrap();
        state.refuse_request_once = true;
    }
    let (session, shutdown, task) = start(&reference).await;
    let host = SdkHost::new(&session.host).unwrap();
    let report = DolphinLaunch
        .play(&host, Some("intent-first"))
        .await
        .unwrap();
    assert_eq!(report.recipe_id, "recipe-2");
    assert_eq!(report.job_id, "job-1");
    let counters = reference.counters();
    assert_eq!(counters.launch_requests, 2);
    assert_eq!(counters.proposals, 2);
    assert_eq!(counters.jobs, 1);
    stop(session, shutdown, task).await;
}

#[tokio::test]
async fn every_error_path_creates_no_job() {
    async fn assert_no_job(mutate: impl FnOnce(&mut HostState), intent: Option<&str>, code: &str) {
        let reference = fixture();
        mutate(&mut reference.state.lock().unwrap());
        let (session, shutdown, task) = start(&reference).await;
        let host = SdkHost::new(&session.host).unwrap();
        let error = DolphinLaunch.play(&host, intent).await.unwrap_err();
        assert_eq!(error.code, code, "{error}");
        assert_eq!(
            reference.counters().jobs,
            0,
            "an error path must not create a launch job"
        );
        stop(session, shutdown, task).await;
    }

    assert_no_job(
        |state| state.entry.game_id.clear(),
        Some("intent"),
        "unmatched",
    )
    .await;
    assert_no_job(
        |state| state.deny_files = true,
        Some("intent"),
        "permission_denied",
    )
    .await;
    assert_no_job(
        |state| state.deny_runtime = true,
        Some("intent"),
        "permission_denied",
    )
    .await;
    assert_no_job(|state| state.files.clear(), Some("intent"), "rom_not_found").await;
    assert_no_job(
        |state| state.deny_request = true,
        Some("intent"),
        "permission_denied",
    )
    .await;
    assert_no_job(|_| {}, None, "no_intent").await;
}

#[tokio::test]
async fn readiness_handlers_never_launch() {
    let reference = fixture();
    let (session, shutdown, task) = start(&reference).await;
    let plugin = DolphinLaunch;
    for operation in [ProviderOperation::Initialize, ProviderOperation::Check] {
        let result = plugin
            .handle(
                ProviderCommand {
                    operation: operation as i32,
                    ..Default::default()
                },
                session.host.clone(),
            )
            .await
            .unwrap();
        assert_eq!(result.disposition, OperationDisposition::Completed as i32);
    }
    assert!(reference.calls().is_empty());
    assert_eq!(reference.state.lock().unwrap().jobs, 0);
    stop(session, shutdown, task).await;
}
