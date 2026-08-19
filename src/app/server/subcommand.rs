//! Host lifecycle ops dispatched from the CLI `serve` verb.
//!
//! Bare `xylitol serve` binds the listener. `stop` prints how to SIGTERM it.
//! There is no lock-file mutex and no `server` / `run` alias.

use clap::Subcommand;

/// Optional leaves under `xylitol serve` (omit to listen).
#[derive(Subcommand, Debug)]
pub enum ServeAction {
    /// Register as a launchd/systemd service (macOS/Linux). Not implemented.
    Install,
    /// Print how to stop a running listener (SIGTERM). Does not read a lock file.
    Stop,
}

/// Run a `serve` invocation.
pub async fn run(
    host: String,
    port: u16,
    action: Option<ServeAction>,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        None => {
            let config = crate::app::server::runtime::ServerConfig {
                host: host.clone(),
                port,
                ..Default::default()
            };
            let (handle, actual_port) = crate::app::server::runtime::start(config).await?;
            eprintln!("Host listening on {host}:{actual_port}");
            tokio::signal::ctrl_c().await?;
            eprintln!("Shutting down...");
            handle.shutdown();
            Ok(())
        }
        Some(ServeAction::Install) => {
            eprintln!("Host install not yet implemented");
            Ok(())
        }
        Some(ServeAction::Stop) => {
            eprintln!(
                "No lock-file stop protocol. Send SIGTERM to the xylitol serve process \
                 (the one listening on the configured host:port, default 127.0.0.1:18790)."
            );
            Ok(())
        }
    }
}
