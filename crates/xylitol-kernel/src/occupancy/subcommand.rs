use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum ServeAction {
    Stop,
}

pub async fn run(
    host: String,
    port: u16,
    action: Option<ServeAction>,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        None => {
            let config = crate::occupancy::runtime::ServerConfig {
                host: host.clone(),
                port,
                ..Default::default()
            };
            let (mut handle, actual_port) = crate::occupancy::runtime::start(config).await?;
            eprintln!("Host listening on {host}:{actual_port}");
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    eprintln!("Shutting down...");
                    handle.shutdown();
                }
                _ = handle.evicted() => {
                    eprintln!("Serve registration taken over by a newer daemon; exiting.");
                    handle.shutdown();
                }
            }
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
