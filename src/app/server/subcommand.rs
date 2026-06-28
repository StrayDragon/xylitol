//! Server lifecycle subcommands dispatched from the CLI `server` verb.
//!
//! `ServerSubcommand` (Run/Install/Stop) and its `run` implementation live here
//! — in the server domain, next to [`crate::app::server::lock`] — rather than in
//! the CLI. The CLI keeps only the `server` parsing variant (`CliCommand::Server`)
//! and delegates here, so server lifecycle logic (lock-file probing, SIGTERM)
//! does not leak into CLI parsing code (design c310 §A/B).

use clap::Subcommand;

/// Server lifecycle subcommands.
#[derive(Subcommand, Debug)]
pub enum ServerSubcommand {
    /// Start the xylitol server.
    Run {
        /// Port to bind to.
        #[arg(long, default_value = "8080")]
        port: u16,
    },
    /// Register the server as a launchd/systemd service (macOS/Linux).
    Install,
    /// Stop a running server by removing its lock file.
    Stop {
        /// Path to the lock file.
        #[arg(long, default_value = "/tmp/xylitol-server.lock")]
        lock: String,
    },
}

/// Run a server subcommand.
pub async fn run(action: ServerSubcommand) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        ServerSubcommand::Run { port } => {
            let config = crate::app::server::runtime::ServerConfig {
                port,
                ..Default::default()
            };
            let (_handle, actual_port) = crate::app::server::runtime::start(config).await?;
            eprintln!("Server started on port {}", actual_port);
            // Keep running until Ctrl+C
            tokio::signal::ctrl_c().await?;
            eprintln!("Shutting down...");
            Ok(())
        }
        ServerSubcommand::Install => {
            eprintln!("Server install not yet implemented");
            Ok(())
        }
        ServerSubcommand::Stop { lock } => {
            let path = std::path::Path::new(&lock);
            if !path.exists() {
                eprintln!("No lock file found at: {lock}");
                return Ok(());
            }

            // Read lock file to get the PID
            match crate::app::server::lock::ServerLock::probe(path) {
                Ok(info) => {
                    eprintln!(
                        "Sending SIGTERM to server (pid {}, port {})",
                        info.pid, info.port
                    );
                    #[cfg(unix)]
                    {
                        use std::process::Command;
                        let _ = Command::new("kill")
                            .arg("-TERM")
                            .arg(info.pid.to_string())
                            .status();
                        // Give it a moment, then remove the lock
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                    #[cfg(not(unix))]
                    {
                        eprintln!("Warning: server stop requires Unix (SIGTERM)");
                    }
                }
                Err(e) => {
                    eprintln!("Could not read lock file: {e}");
                }
            }

            // Clean up lock file
            std::fs::remove_file(path).ok();
            eprintln!("Lock file removed: {lock}");
            Ok(())
        }
    }
}
