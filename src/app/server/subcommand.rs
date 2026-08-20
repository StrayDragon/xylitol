//! Server lifecycle subcommands dispatched from the CLI `server` verb.
//!
//! `stop` prints how to SIGTERM the listener. There is no lock-file mutex.

use clap::Subcommand;

/// Server lifecycle subcommands.
#[derive(Subcommand, Debug)]
pub enum ServerSubcommand {
    /// Start the xylitol server.
    Run {
        /// Port to bind to.
        #[arg(long, default_value = "18790")]
        port: u16,
    },
    /// Register the server as a launchd/systemd service (macOS/Linux).
    Install,
    /// Print how to stop a running listener (SIGTERM). Does not read a lock file.
    Stop {
        /// Ignored (legacy flag; lock files are not a product mutex).
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
            let (handle, actual_port) = crate::app::server::runtime::start(config).await?;
            eprintln!("Server started on port {}", actual_port);
            tokio::signal::ctrl_c().await?;
            eprintln!("Shutting down...");
            handle.shutdown();
            Ok(())
        }
        ServerSubcommand::Install => {
            eprintln!("Server install not yet implemented");
            Ok(())
        }
        ServerSubcommand::Stop { lock: _ } => {
            eprintln!(
                "No lock-file stop protocol. Send SIGTERM to the xylitol server process \
                 (the one listening on the configured host:port, default 127.0.0.1:18790)."
            );
            Ok(())
        }
    }
}
