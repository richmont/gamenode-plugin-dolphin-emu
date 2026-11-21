use std::sync::Arc;

use gamenode_dolphin_launch::{CAPABILITIES, DolphinLaunch, PLUGIN_ID, PLUGIN_VERSION};
use gamenode_plugin_sdk::{Launch, Session};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let launch = Launch::from_env()?;
    let session =
        Session::open_extension_from_launch(launch, PLUGIN_ID, PLUGIN_VERSION, CAPABILITIES)
            .await?;
    gamenode_plugin_sdk::run_extension(session, Arc::new(DolphinLaunch)).await?;
    Ok(())
}
