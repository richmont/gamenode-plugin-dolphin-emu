use std::sync::Arc;

use gamenode_dolphin_emu_launch::{
    CAPABILITIES, DolphinLaunch, LaunchHost, PLUGIN_ID, PLUGIN_VERSION, SdkHost,
};
use gamenode_plugin_sdk::{Launch, Session};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let launch = Launch::from_env()?;
    // Read this before consuming the launch: the host sets it only for a user-initiated action.
    // A readiness startup has no intent and must never launch.
    let intent = launch.user_intent_id();
    let session =
        Session::open_extension_from_launch(launch, PLUGIN_ID, PLUGIN_VERSION, CAPABILITIES)
            .await?;
    let Some(intent) = intent else {
        // Readiness sessions are one-shot: report session readiness and exit without launching.
        return Ok(());
    };
    let result = {
        let host = SdkHost::new(&session.host)?;
        DolphinLaunch.play(&host, Some(&intent)).await
    };
    match result {
        Ok(_report) => {
            // Keep the authenticated session open while the host's launch job rechecks the
            // approval fence. The host shuts this process down when the action completes; it also
            // has its own five-minute extension limit.
            gamenode_plugin_sdk::run_extension(session, Arc::new(DolphinLaunch)).await?;
            Ok(())
        }
        Err(error) => {
            // stderr is discarded by the supervised host; a notification is the user-visible
            // channel when the optional notifications.publish grant is present.
            if let Ok(host) = SdkHost::new(&session.host) {
                let _ = host.notify_failure(&error).await;
            }
            Err(Box::new(error) as Box<dyn std::error::Error>)
        }
    }
}
