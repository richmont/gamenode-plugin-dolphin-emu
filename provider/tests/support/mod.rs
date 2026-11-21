// Public SDK reference-host protocol stubs. Unused RPCs explicitly reject calls.
use super::*;
#[tonic::async_trait]
impl plugin_host_server::PluginHost for ReferenceHost {
    type WatchPermissionChangesStream =
        tokio_stream::wrappers::ReceiverStream<Result<PermissionChange, Status>>;
    async fn open_session(
        &self,
        request: tonic::Request<OpenSessionRequest>,
    ) -> std::result::Result<tonic::Response<OpenSessionReply>, tonic::Status> {
        self.open_session_impl(request).await
    }
    async fn submit_discovery(
        &self,
        _request: tonic::Request<SubmitDiscoveryRequest>,
    ) -> std::result::Result<tonic::Response<SubmitDiscoveryReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn describe_api(
        &self,
        _request: tonic::Request<DescribeApiRequest>,
    ) -> std::result::Result<tonic::Response<DescribeApiReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn get_permissions(
        &self,
        _request: tonic::Request<GetPermissionsRequest>,
    ) -> std::result::Result<tonic::Response<GetPermissionsReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn watch_permission_changes(
        &self,
        _request: tonic::Request<WatchPermissionChangesRequest>,
    ) -> std::result::Result<tonic::Response<Self::WatchPermissionChangesStream>, tonic::Status>
    {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
}
#[tonic::async_trait]
impl provider_host_server::ProviderHost for ReferenceHost {
    type ExchangeStream = tokio_stream::wrappers::ReceiverStream<Result<ProviderCommand, Status>>;
    async fn exchange(
        &self,
        request: tonic::Request<tonic::Streaming<ProviderEvent>>,
    ) -> std::result::Result<tonic::Response<Self::ExchangeStream>, tonic::Status> {
        self.exchange_impl(request).await
    }
    async fn get_context(
        &self,
        request: tonic::Request<ServiceContext>,
    ) -> std::result::Result<tonic::Response<ProviderContext>, tonic::Status> {
        self.get_context_impl(request).await
    }
    async fn account(
        &self,
        _request: tonic::Request<AccountRequest>,
    ) -> std::result::Result<tonic::Response<AccountReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn secret(
        &self,
        _request: tonic::Request<SecretRequest>,
    ) -> std::result::Result<tonic::Response<SecretReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn login(
        &self,
        _request: tonic::Request<LoginRequest>,
    ) -> std::result::Result<tonic::Response<LoginReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn declare_actions(
        &self,
        _request: tonic::Request<ActionRequest>,
    ) -> std::result::Result<tonic::Response<ActionReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
}
#[tonic::async_trait]
impl resource_host_server::ResourceHost for ReferenceHost {
    type LibraryWatchStream = tokio_stream::wrappers::ReceiverStream<Result<LibraryEvent, Status>>;
    type LibrariesWatchStream =
        tokio_stream::wrappers::ReceiverStream<Result<LibraryRecordEvent, Status>>;
    type RuntimesWatchStream =
        tokio_stream::wrappers::ReceiverStream<Result<RuntimeRecordEvent, Status>>;
    type DataWatchStream = tokio_stream::wrappers::ReceiverStream<Result<DataEvent, Status>>;
    type JobsWatchStream = tokio_stream::wrappers::ReceiverStream<Result<JobEvent, Status>>;
    async fn library_list(
        &self,
        _request: tonic::Request<LibraryListRequest>,
    ) -> std::result::Result<tonic::Response<LibraryListReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn library_get(
        &self,
        _request: tonic::Request<LibraryGetRequest>,
    ) -> std::result::Result<tonic::Response<LibraryGetReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn library_watch(
        &self,
        _request: tonic::Request<LibraryWatchRequest>,
    ) -> std::result::Result<tonic::Response<Self::LibraryWatchStream>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn metadata_update(
        &self,
        _request: tonic::Request<MetadataUpdateRequest>,
    ) -> std::result::Result<tonic::Response<MutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn favorites_list(
        &self,
        _request: tonic::Request<FavoritesListRequest>,
    ) -> std::result::Result<tonic::Response<FavoritesListReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn favorites_set(
        &self,
        _request: tonic::Request<FavoritesSetRequest>,
    ) -> std::result::Result<tonic::Response<MutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn matching_search(
        &self,
        _request: tonic::Request<MatchingSearchRequest>,
    ) -> std::result::Result<tonic::Response<MatchingSearchReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn matching_assign(
        &self,
        _request: tonic::Request<MatchingAssignRequest>,
    ) -> std::result::Result<tonic::Response<MutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn artwork_read(
        &self,
        _request: tonic::Request<ArtworkReadRequest>,
    ) -> std::result::Result<tonic::Response<ArtworkReadReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn artwork_set(
        &self,
        _request: tonic::Request<ArtworkSetRequest>,
    ) -> std::result::Result<tonic::Response<MutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn games_request_launch(
        &self,
        _request: tonic::Request<GameLaunchRequest>,
    ) -> std::result::Result<tonic::Response<GameLaunchReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn games_propose_custom_launch(
        &self,
        _request: tonic::Request<CustomLaunchRequest>,
    ) -> std::result::Result<tonic::Response<CustomLaunchReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn games_request_custom_launch(
        &self,
        _request: tonic::Request<CustomLaunchRequest>,
    ) -> std::result::Result<tonic::Response<CustomLaunchReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn games_remove_custom_launch(
        &self,
        _request: tonic::Request<CustomLaunchRemoveRequest>,
    ) -> std::result::Result<tonic::Response<MutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn games_request_install(
        &self,
        _request: tonic::Request<GameInstallRequest>,
    ) -> std::result::Result<tonic::Response<JobReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn storage_get(
        &self,
        _request: tonic::Request<StorageGetRequest>,
    ) -> std::result::Result<tonic::Response<StorageGetReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn storage_put(
        &self,
        _request: tonic::Request<StoragePutRequest>,
    ) -> std::result::Result<tonic::Response<StorageMutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn storage_delete(
        &self,
        _request: tonic::Request<StorageDeleteRequest>,
    ) -> std::result::Result<tonic::Response<MutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn files_request_access(
        &self,
        _request: tonic::Request<FilesRequestAccessRequest>,
    ) -> std::result::Result<tonic::Response<FileAccessReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn files_list(
        &self,
        _request: tonic::Request<FilesListRequest>,
    ) -> std::result::Result<tonic::Response<FilesListReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn files_read(
        &self,
        _request: tonic::Request<FilesReadRequest>,
    ) -> std::result::Result<tonic::Response<FileReadReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn files_write(
        &self,
        _request: tonic::Request<FilesWriteRequest>,
    ) -> std::result::Result<tonic::Response<MutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn files_delete(
        &self,
        _request: tonic::Request<FilesDeleteRequest>,
    ) -> std::result::Result<tonic::Response<MutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn network_fetch(
        &self,
        _request: tonic::Request<NetworkFetchRequest>,
    ) -> std::result::Result<tonic::Response<NetworkFetchReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn notifications_publish(
        &self,
        _request: tonic::Request<NotificationPublishRequest>,
    ) -> std::result::Result<tonic::Response<MutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn backups_list(
        &self,
        _request: tonic::Request<BackupsListRequest>,
    ) -> std::result::Result<tonic::Response<BackupsListReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn backups_request_create(
        &self,
        _request: tonic::Request<BackupCreateRequest>,
    ) -> std::result::Result<tonic::Response<JobReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn backups_request_restore(
        &self,
        _request: tonic::Request<BackupRestoreRequest>,
    ) -> std::result::Result<tonic::Response<JobReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn libraries_list(
        &self,
        _request: tonic::Request<DomainListRequest>,
    ) -> std::result::Result<tonic::Response<LibrariesListReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn libraries_get(
        &self,
        _request: tonic::Request<DomainGetRequest>,
    ) -> std::result::Result<tonic::Response<LibraryRecordReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn libraries_watch(
        &self,
        _request: tonic::Request<DomainWatchRequest>,
    ) -> std::result::Result<tonic::Response<Self::LibrariesWatchStream>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn libraries_create(
        &self,
        _request: tonic::Request<LibraryCreateRequest>,
    ) -> std::result::Result<tonic::Response<LibraryMutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn libraries_update(
        &self,
        _request: tonic::Request<LibraryUpdateRequest>,
    ) -> std::result::Result<tonic::Response<LibraryMutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn libraries_remove(
        &self,
        _request: tonic::Request<DomainRemoveRequest>,
    ) -> std::result::Result<tonic::Response<MutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn libraries_add_root(
        &self,
        _request: tonic::Request<LibraryRootAddRequest>,
    ) -> std::result::Result<tonic::Response<LibraryMutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn libraries_update_root(
        &self,
        _request: tonic::Request<LibraryRootUpdateRequest>,
    ) -> std::result::Result<tonic::Response<LibraryMutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn libraries_remove_root(
        &self,
        _request: tonic::Request<LibraryRootRemoveRequest>,
    ) -> std::result::Result<tonic::Response<MutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn runtimes_list(
        &self,
        _request: tonic::Request<DomainListRequest>,
    ) -> std::result::Result<tonic::Response<RuntimesListReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn runtimes_get(
        &self,
        _request: tonic::Request<DomainGetRequest>,
    ) -> std::result::Result<tonic::Response<RuntimeRecordReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn runtimes_watch(
        &self,
        _request: tonic::Request<DomainWatchRequest>,
    ) -> std::result::Result<tonic::Response<Self::RuntimesWatchStream>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn runtimes_register(
        &self,
        _request: tonic::Request<RuntimeRegisterRequest>,
    ) -> std::result::Result<tonic::Response<RuntimeMutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn runtimes_update(
        &self,
        _request: tonic::Request<RuntimeUpdateRequest>,
    ) -> std::result::Result<tonic::Response<RuntimeMutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn runtimes_remove(
        &self,
        _request: tonic::Request<DomainRemoveRequest>,
    ) -> std::result::Result<tonic::Response<MutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn games_register_content(
        &self,
        _request: tonic::Request<ContentRegistrationRequest>,
    ) -> std::result::Result<tonic::Response<MutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn storage_diagnostics_publish_benchmark(
        &self,
        _request: tonic::Request<BenchmarkReportRequest>,
    ) -> std::result::Result<tonic::Response<ReportMutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn storage_diagnostics_publish_smart(
        &self,
        _request: tonic::Request<SmartReportRequest>,
    ) -> std::result::Result<tonic::Response<ReportMutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn computer_publish_hardware(
        &self,
        _request: tonic::Request<HardwareReportRequest>,
    ) -> std::result::Result<tonic::Response<ReportMutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn game_presence_publish(
        &self,
        _request: tonic::Request<PresencePublishRequest>,
    ) -> std::result::Result<tonic::Response<ReportMutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn game_presence_clear(
        &self,
        _request: tonic::Request<PresenceClearRequest>,
    ) -> std::result::Result<tonic::Response<MutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn game_performance_publish_samples(
        &self,
        _request: tonic::Request<PerformanceSamplesRequest>,
    ) -> std::result::Result<tonic::Response<ReportMutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn game_performance_end_series(
        &self,
        _request: tonic::Request<PerformanceEndSeriesRequest>,
    ) -> std::result::Result<tonic::Response<MutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn data_publish(
        &self,
        request: tonic::Request<DataPublishRequest>,
    ) -> std::result::Result<tonic::Response<DataMutationReply>, tonic::Status> {
        self.data_publish_impl(request).await
    }
    async fn data_get(
        &self,
        request: tonic::Request<DataGetRequest>,
    ) -> std::result::Result<tonic::Response<DataGetReply>, tonic::Status> {
        self.data_get_impl(request).await
    }
    async fn data_list(
        &self,
        _request: tonic::Request<DataListRequest>,
    ) -> std::result::Result<tonic::Response<DataListReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn data_watch(
        &self,
        _request: tonic::Request<DataWatchRequest>,
    ) -> std::result::Result<tonic::Response<Self::DataWatchStream>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn persistence_get_usage(
        &self,
        _request: tonic::Request<PersistenceUsageRequest>,
    ) -> std::result::Result<tonic::Response<PersistenceUsageReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn persistence_list_entries(
        &self,
        _request: tonic::Request<PersistenceListRequest>,
    ) -> std::result::Result<tonic::Response<PersistenceListReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn persistence_delete_entries(
        &self,
        _request: tonic::Request<PersistenceDeleteRequest>,
    ) -> std::result::Result<tonic::Response<MutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn persistence_clear(
        &self,
        _request: tonic::Request<PersistenceClearRequest>,
    ) -> std::result::Result<tonic::Response<MutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn persistence_set_retention_policy(
        &self,
        _request: tonic::Request<RetentionPolicyRequest>,
    ) -> std::result::Result<tonic::Response<MutationReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn tools_list(
        &self,
        _request: tonic::Request<ToolsListRequest>,
    ) -> std::result::Result<tonic::Response<ToolsListReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn tools_request_apply(
        &self,
        _request: tonic::Request<ToolApplyRequest>,
    ) -> std::result::Result<tonic::Response<JobReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn tools_request_remove(
        &self,
        _request: tonic::Request<ToolRemoveRequest>,
    ) -> std::result::Result<tonic::Response<JobReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn ports_list(
        &self,
        _request: tonic::Request<PortsListRequest>,
    ) -> std::result::Result<tonic::Response<PortsListReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn ports_detail(
        &self,
        _request: tonic::Request<PortDetailRequest>,
    ) -> std::result::Result<tonic::Response<PortDetailReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn ports_request_install(
        &self,
        _request: tonic::Request<PortInstallRequest>,
    ) -> std::result::Result<tonic::Response<JobReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn jobs_get(
        &self,
        _request: tonic::Request<JobGetRequest>,
    ) -> std::result::Result<tonic::Response<JobReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn jobs_watch(
        &self,
        _request: tonic::Request<JobWatchRequest>,
    ) -> std::result::Result<tonic::Response<Self::JobsWatchStream>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
    async fn jobs_cancel(
        &self,
        _request: tonic::Request<JobCancelRequest>,
    ) -> std::result::Result<tonic::Response<JobReply>, tonic::Status> {
        Err(Status::unimplemented("reference host: unused RPC"))
    }
}
