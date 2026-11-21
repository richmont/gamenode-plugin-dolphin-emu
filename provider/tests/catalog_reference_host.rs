//! Independent gRPC reference host using only public SDK messages and service traits.
//! Disk persistence here tests the client contract, not GameNode's production storage engine.
use gamenode_dolphin_provider::{CAPABILITIES, DolphinProvider, PLUGIN_ID, PROVIDER_ID, catalog};
use gamenode_plugin_sdk::{Error, Launch, ProviderPlugin, Session, SessionOptions, proto::*};
use prost::Message;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio_stream::wrappers::{ReceiverStream, TcpListenerStream};
use tonic::{Request, Response, Status};
mod support;

#[derive(Clone)]
struct ReferenceHost {
    publish_allowed: bool,
    persist_allowed: bool,
    root: PathBuf,
    requests: Arc<Mutex<Vec<DataPublishRequest>>>,
}
impl ReferenceHost {
    async fn open_session_impl(
        &self,
        r: Request<OpenSessionRequest>,
    ) -> Result<Response<OpenSessionReply>, Status> {
        let r = r.into_inner();
        Ok(Response::new(OpenSessionReply {
            session_id: "reference".into(),
            api_major: 1,
            api_minor: 6,
            granted_capabilities: r.requested_capabilities,
            resource_context: Some(ServiceContext {
                session_id: "reference".into(),
                session_credential: r.session_credential,
                provider_id: PROVIDER_ID.into(),
                session_kind: SessionKind::Provider as i32,
                ..Default::default()
            }),
            ..Default::default()
        }))
    }
    async fn exchange_impl(
        &self,
        _r: Request<tonic::Streaming<ProviderEvent>>,
    ) -> Result<Response<ReceiverStream<Result<ProviderCommand, Status>>>, Status> {
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
    async fn get_context_impl(
        &self,
        _r: Request<ServiceContext>,
    ) -> Result<Response<ProviderContext>, Status> {
        Ok(Response::new(ProviderContext {
            provider_id: PROVIDER_ID.into(),
            ..Default::default()
        }))
    }
    async fn data_publish_impl(
        &self,
        r: Request<DataPublishRequest>,
    ) -> Result<Response<DataMutationReply>, Status> {
        let r = r.into_inner();
        self.requests.lock().unwrap().push(r.clone());
        let mutation = r.mutation.as_ref().unwrap();
        assert_eq!(mutation.expected_revision, 0);
        assert!(!mutation.idempotency_key.is_empty());
        let context = mutation.context.as_ref().unwrap();
        assert_eq!(context.session_credential, vec![9; 32]);
        assert_eq!(r.durability, StorageDurability::Persistent as i32);
        if !self.publish_allowed {
            return Err(Status::permission_denied("data.publish"));
        }
        if !self.persist_allowed {
            return Err(Status::permission_denied("storage.persist"));
        }
        assert_eq!(r.dataset_id, catalog::DATASET);
        assert_eq!(r.schema_id, catalog::DATASET);
        assert_eq!(r.schema_version, 1);
        assert!(r.opaque_payload.is_empty());
        // This fixture has one authenticated owner; each resource gets a separate durable row.
        assert!(
            r.resource_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        );
        let file = self.root.join(&r.resource_id);
        let previous = std::fs::read(&file)
            .ok()
            .map(|b| DataRecord::decode(b.as_slice()).unwrap());
        let revision = previous.map_or(1, |r| r.revision + 1);
        let record = DataRecord {
            record_id: r.resource_id.clone(),
            dataset_id: r.dataset_id,
            schema_id: r.schema_id,
            schema_version: r.schema_version,
            revision,
            value: r.value,
            durability: r.durability,
            ..Default::default()
        };
        std::fs::write(file, record.encode_to_vec()).unwrap();
        Ok(Response::new(DataMutationReply {
            record_id: record.record_id,
            revision,
            durability: r.durability,
            outcome: MutationOutcome::Applied as i32,
            ..Default::default()
        }))
    }
    async fn data_get_impl(
        &self,
        r: Request<DataGetRequest>,
    ) -> Result<Response<DataGetReply>, Status> {
        let r = r.into_inner();
        assert_eq!(r.dataset_id, catalog::DATASET);
        if !r
            .record_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(Status::invalid_argument("id"));
        }
        let bytes =
            std::fs::read(self.root.join(r.record_id)).map_err(|_| Status::not_found("missing"))?;
        Ok(Response::new(DataGetReply {
            record: Some(DataRecord::decode(bytes.as_slice()).unwrap()),
        }))
    }
}
async fn connect(
    host: ReferenceHost,
) -> (
    Session,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel();
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
            operation_id: "discover".into(),
        },
        PLUGIN_ID,
        PROVIDER_ID,
        "0.1.0",
        CAPABILITIES,
        SessionOptions {
            min_api_major: 1,
            min_api_minor: 6,
            max_api_major: 1,
            max_api_minor: 6,
            session_kind: SessionKind::Provider,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    (session, stop, task)
}
fn reference(root: PathBuf, publish_allowed: bool, persist_allowed: bool) -> ReferenceHost {
    ReferenceHost {
        root,
        publish_allowed,
        persist_allowed,
        requests: Default::default(),
    }
}
fn command() -> ProviderCommand {
    ProviderCommand {
        operation: ProviderOperation::Discover as i32,
        ..Default::default()
    }
}

#[tokio::test]
async fn durable_catalog_replaces_on_rescan_and_survives_reference_host_restart() {
    let disk = tempfile::tempdir().unwrap();
    let roms = tempfile::tempdir().unwrap();
    std::fs::write(roms.path().join("Zelda.iso"), b"GZLE01").unwrap();
    let provider = DolphinProvider::new(vec![roms.path().into()]);
    let reference = reference(disk.path().into(), true, true);
    let (session, stop, task) = connect(reference.clone()).await;
    for revision in 1..=2 {
        let result = provider
            .handle(command(), session.host.clone())
            .await
            .unwrap();
        assert_eq!(result.discovery.unwrap().records.len(), 1);
        let record = session
            .host
            .resources()
            .unwrap()
            .data_get(catalog::DATASET, "GZLE01", None)
            .await
            .unwrap();
        assert_eq!(record.revision, revision);
        assert_eq!(record.durability, StorageDurability::Persistent as i32);
    }
    assert_eq!(std::fs::read_dir(disk.path()).unwrap().count(), 1);
    let requests = reference.requests.lock().unwrap().clone();
    let payload = requests[0].value.as_ref().unwrap();
    assert_eq!(
        payload
            .fields
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["content_id", "disc_id", "platform", "title"]
    );
    let values: BTreeMap<_, _> = payload
        .fields
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let printed = format!("{values:?}");
    assert!(!printed.contains(roms.path().to_str().unwrap()));
    assert!(!printed.contains("credential"));
    assert!(!printed.contains("Zelda.iso"));
    assert_eq!(requests[0].resource_id, requests[1].resource_id);
    assert_ne!(
        requests[0].source_observation_id,
        requests[1].source_observation_id
    );
    drop(requests);
    drop(session);
    stop.send(()).unwrap();
    task.await.unwrap();
    // Fresh reference host and channel, using only the on-disk fixture state.
    let (consumer, stop, task) = connect(ReferenceHost {
        requests: Default::default(),
        ..reference
    })
    .await;
    let row = consumer
        .host
        .resources()
        .unwrap()
        .data_get(catalog::DATASET, "GZLE01", None)
        .await
        .unwrap();
    assert_eq!(row.revision, 2);
    drop(consumer);
    stop.send(()).unwrap();
    task.await.unwrap();
}

async fn denied(publish: bool, persist: bool) {
    let disk = tempfile::tempdir().unwrap();
    let roms = tempfile::tempdir().unwrap();
    std::fs::write(roms.path().join("Mario.iso"), b"RMCE01").unwrap();
    let provider = DolphinProvider::new(vec![roms.path().into()]);
    let reference = reference(disk.path().into(), publish, persist);
    let (session, stop, task) = connect(reference.clone()).await;
    assert_eq!(
        catalog::publish(&session.host, &provider.discovery().records).await,
        Err(Error::Denied)
    );
    let result = provider
        .handle(command(), session.host.clone())
        .await
        .unwrap();
    assert_eq!(result.discovery.unwrap().records.len(), 1);
    assert_eq!(std::fs::read_dir(disk.path()).unwrap().count(), 0);
    assert!(
        reference
            .requests
            .lock()
            .unwrap()
            .iter()
            .all(|r| r.durability == StorageDurability::Persistent as i32)
    );
    drop(session);
    stop.send(()).unwrap();
    task.await.unwrap();
}
#[tokio::test]
async fn missing_persistence_never_downgrades_or_loses_discovery() {
    denied(true, false).await;
}
#[tokio::test]
async fn missing_publish_never_creates_a_row_or_loses_discovery() {
    denied(false, true).await;
}

#[test]
fn unknown_discs_use_the_opaque_scanner_identity_and_keep_unknown_platform() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("Homebrew.iso"), [0; 6]).unwrap();
    let record = DolphinProvider::new(vec![dir.path().into()])
        .discovery()
        .records
        .remove(0);
    let request = catalog::publication(&record);
    assert_eq!(request.resource_id, record.id);
    assert!(request.resource_id.starts_with("dolphin-"));
    assert_eq!(
        request.value.unwrap().fields["platform"].kind,
        Some(gamenode_plugin_sdk::prost_types::value::Kind::StringValue(
            "unknown".into()
        ))
    );
}
