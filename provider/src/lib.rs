//! GameNode provider for GameCube and Wii games managed by Dolphin.
//!
//! Discovery only: this provider lists the games Dolphin already knows about. Starting them is
//! the job of the sibling `gamenode-dolphin-emu-launch` extension, because a custom launch needs the
//! host's runtime/consent flow and `game.custom_launch` grant.

pub mod catalog;
pub mod roms;

use std::path::PathBuf;

use gamenode_plugin_sdk::{
    Error, HostClient, ProviderPlugin, capability,
    proto::{
        DiscoveryResult, OperationDisposition, OperationResult, ProviderCommand, ProviderContext,
        ProviderOperation,
    },
};

pub const PLUGIN_ID: &str = "dev.richmont.gamenode.dolphin-emu";
pub const PROVIDER_ID: &str = "dolphin-emu";
pub const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const CAPABILITIES: &[&str] = &[
    capability::CONTEXT,
    capability::DISCOVERY,
    capability::DATA_PUBLISH,
    capability::STORAGE_PERSIST,
];

/// A provider that scans the folders Dolphin itself is configured to use.
#[derive(Clone)]
pub struct DolphinProvider {
    roots: Vec<PathBuf>,
}

impl DolphinProvider {
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self { roots }
    }

    /// Reads Dolphin's `ISOPath*` entries plus `DOLPHIN_PLUGIN_ROOTS`.
    pub fn from_environment() -> Self {
        Self::new(roms::roots())
    }

    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    pub fn discovery(&self) -> DiscoveryResult {
        DiscoveryResult {
            provider_id: PROVIDER_ID.into(),
            records: roms::scan(&self.roots),
            ..Default::default()
        }
    }

    async fn context(host: &HostClient) -> Result<ProviderContext, Error> {
        let context = host.provider_context().await?;
        if context.provider_id != PROVIDER_ID {
            return Err(Error::Protocol);
        }
        Ok(context)
    }
}

#[async_trait::async_trait]
impl ProviderPlugin for DolphinProvider {
    async fn handle(
        &self,
        command: ProviderCommand,
        host: HostClient,
    ) -> Result<OperationResult, Error> {
        Self::context(&host).await?;
        let mut result = OperationResult {
            disposition: OperationDisposition::Completed as i32,
            ..Default::default()
        };
        match ProviderOperation::try_from(command.operation).map_err(|_| Error::Protocol)? {
            ProviderOperation::Initialize | ProviderOperation::Check => {}
            ProviderOperation::Discover => {
                // Discovery walks the filesystem; keep the provider stream free.
                let provider = self.clone();
                result.discovery = Some(
                    tokio::task::spawn_blocking(move || provider.discovery())
                        .await
                        .map_err(|_| Error::Protocol)?,
                );
                if let Some(discovery) = &result.discovery {
                    if let Err(error) = catalog::publish(&host, &discovery.records).await {
                        // SDK errors contain safe categories, never server status text or paths.
                        eprintln!("dolphin-emu catalog publication: {error}");
                    }
                }
            }
            _ => return Err(Error::Protocol),
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn discovery_reports_the_scanned_roms_under_the_dolphin_provider() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("Zelda.iso"), b"GZLE01\x00\x00").unwrap();
        let provider = DolphinProvider::new(vec![directory.path().to_path_buf()]);
        let discovery = provider.discovery();
        assert_eq!(discovery.provider_id, "dolphin-emu");
        assert_eq!(discovery.records.len(), 1);
        assert_eq!(discovery.records[0].id, "GZLE01");
    }
}
