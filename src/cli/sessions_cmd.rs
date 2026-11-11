use std::path::PathBuf;

use crate::cli::args::SessionsAction;
use xylitol_kernel::config::loader::load_app_config_detailed;

pub async fn run(action: SessionsAction) -> Result<(), String> {
    match action {
        SessionsAction::Maintain { dir } => {
            let dir = match dir {
                Some(dir) => PathBuf::from(dir),
                None => {
                    let loaded =
                        load_app_config_detailed(None).map_err(|error| error.to_string())?;
                    let config: xylitol_official_plugin_session::plugin::SessionPluginConfig =
                        loaded
                            .config
                            .section_or_default("session")
                            .unwrap_or_default();
                    config.sessions_dir.map(PathBuf::from).unwrap_or_else(
                        xylitol_official_plugin_session::LogSessionStore::default_dir,
                    )
                }
            };
            let report = xylitol_official_plugin_session::maintain_sessions(dir)
                .await
                .map_err(|error| error.to_string())?;
            println!("maintained {} session(s)", report.sessions.len());
            for session in &report.sessions {
                println!(
                    "{}: files={} bytes={} entries={} suggested_threshold_bytes={}",
                    session.id,
                    session.files,
                    session.bytes,
                    session.entries,
                    session.suggested_threshold_bytes
                );
            }
            Ok(())
        }
    }
}
