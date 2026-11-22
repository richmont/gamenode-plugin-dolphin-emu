//! GameNode extension that starts a GameCube/Wii game in Dolphin.
//!
//! The host owns process supervision: it launches the runtime, holds the resource leases and
//! reports the session as ended when the process tree exits. This extension only supplies the
//! approved runtime recipe.
//!
//! ## Current state
//!
//! The pure recipe/runtime builders are complete and tested. The session flow below is the
//! remaining work and is documented in `TASKS.md`:
//!
//! 1. resolve the Dolphin runtime (registered by the user/host, or register one from an approved
//!    executable handle);
//! 2. bind the selected game to its ROM content handle;
//! 3. propose the recipe once and request the approved revision on later plays, using the host
//!    intent id `GAMENODE_PLUGIN_USER_INTENT_ID`.
//!
//! It deliberately never guesses an executable path or starts a process directly.

use gamenode_plugin_sdk::{
    Error, ExtensionPlugin, HostClient, capability,
    proto::{OperationDisposition, OperationResult, ProviderCommand, ProviderOperation},
    resources::{CustomLaunchRecipe, RuntimeArgumentTemplate, RuntimeContentBinding, RuntimeSpec},
};

pub const PLUGIN_ID: &str = "dev.richmont.gamenode.dolphin-emu-launch";
pub const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Suffix-free content kinds this runtime accepts. They mirror the provider's `platform` field.
pub const SUPPORTED_FORMATS: &[&str] = &["gamecube-rom", "wii-rom"];

pub const CAPABILITIES: &[&str] = &[
    capability::CONTEXT,
    capability::GAME_CUSTOM_LAUNCH,
    capability::GAME_RUNTIME_READ,
    capability::GAME_RUNTIME_CREATE,
    capability::GAME_RUNTIME_UPDATE,
    capability::GAME_RUNTIME_DELETE,
    capability::FILES_READ,
    capability::LIBRARY_READ,
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
    platform: &str,
) -> RuntimeContentBinding {
    RuntimeContentBinding {
        content_id: String::new(),
        game_id: game_id.into(),
        installation_id: installation_id.into(),
        content_handle: content_handle.into(),
        content_kind: "rom".into(),
        runtime_id: runtime_id.into(),
        supported_platforms: vec![platform.into()],
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

#[derive(Default)]
pub struct DolphinLaunch;

#[async_trait::async_trait]
impl ExtensionPlugin for DolphinLaunch {
    async fn handle(
        &self,
        command: ProviderCommand,
        host: HostClient,
    ) -> Result<OperationResult, Error> {
        let operation =
            ProviderOperation::try_from(command.operation).map_err(|_| Error::Protocol)?;
        if matches!(
            operation,
            ProviderOperation::Initialize | ProviderOperation::Check
        ) {
            let target = host.context().target_game.clone();
            if let Some(target) = target {
                // The recipe flow needs the host-approved Dolphin runtime and ROM handles. Keep
                // the session successful while the consent wiring in TASKS.md is pending; never
                // fall back to starting a process from the plugin.
                eprintln!(
                    "dolphin-emu launch: target game {} installation {} is ready for the runtime recipe",
                    target.game_id, target.installation_id
                );
            }
        }
        Ok(OperationResult {
            disposition: OperationDisposition::Completed as i32,
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(runtime.supported_formats, ["gamecube-rom", "wii-rom"]);
    }

    #[test]
    fn a_content_binding_keeps_the_host_identities_and_no_path() {
        let binding = content_binding("GZLE01", "id-1", "rom-handle", "runtime-1", "gamecube");
        assert_eq!(binding.game_id, "GZLE01");
        assert_eq!(binding.installation_id, "id-1");
        assert_eq!(binding.content_handle, "rom-handle");
        assert_eq!(binding.runtime_id, "runtime-1");
        assert_eq!(binding.supported_platforms, ["gamecube"]);
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
}
