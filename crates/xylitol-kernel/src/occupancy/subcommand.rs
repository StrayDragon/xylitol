use std::path::PathBuf;
use std::time::Duration;

use clap::Subcommand;
use xylitol_serve::{ServeLifecycle, parse_idle_duration};

use crate::attach::pid_alive;

#[derive(Subcommand, Debug)]
pub enum ServeAction {
    Stop,
}

pub async fn run(
    host: String,
    port: u16,
    lifecycle: Option<String>,
    idle_timeout: Option<String>,
    action: Option<ServeAction>,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        None => {
            let lifecycle = parse_lifecycle_flag(lifecycle.as_deref())?;
            let idle_timeout = parse_idle_flag(idle_timeout.as_deref())?;
            let config = crate::occupancy::runtime::ServerConfig {
                host: host.clone(),
                port,
                lifecycle,
                idle_timeout,
                ..Default::default()
            };
            let (handle, actual_port) = crate::occupancy::runtime::start(config).await?;
            eprintln!("Host listening on {host}:{actual_port}");
            #[cfg(unix)]
            let terminate = async {
                use tokio::signal::unix::{SignalKind, signal};
                let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM");
                sigterm.recv().await;
            };
            #[cfg(not(unix))]
            let terminate = std::future::pending::<()>();
            let mut evicted_rx = handle.subscribe_evicted();
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    eprintln!("Shutting down...");
                }
                _ = evicted_rx.changed() => {
                    eprintln!("Serve registration taken over by a newer daemon; exiting.");
                }
                _ = handle.last_client_idle() => {
                    eprintln!("Last client disconnected; shutting down.");
                }
                _ = terminate => {
                    eprintln!("Shutting down...");
                }
            }
            handle.shutdown();
            Ok(())
        }
        Some(ServeAction::Stop) => stop_registered_host(None).await,
    }
}

pub fn parse_lifecycle_flag(raw: Option<&str>) -> Result<ServeLifecycle, String> {
    match raw {
        None => Ok(ServeLifecycle::Persistent),
        Some(raw) => ServeLifecycle::parse_name(raw).ok_or_else(|| {
            format!("invalid --lifecycle {raw:?}; expected persistent or last-client")
        }),
    }
}

pub fn parse_idle_flag(raw: Option<&str>) -> Result<Duration, String> {
    match raw {
        None => Ok(xylitol_serve::DEFAULT_IDLE_GRACE),
        Some(raw) => parse_idle_duration(raw)
            .ok_or_else(|| format!("invalid --idle-timeout {raw:?}; expected e.g. 30s or 500ms")),
    }
}

pub async fn stop_registered_host(
    registration_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = registration_path.unwrap_or_else(xylitol_serve::default_registration_path);
    let reg = xylitol_serve::read_registration(&path).map_err(|_| {
        format!(
            "No Host registration at {}. Start one with `xylitol serve`, or it already stopped.",
            path.display()
        )
    })?;
    if reg.pid == std::process::id() {
        return Err(
            "Refusing to stop the current process; `serve stop` targets the registered Host pid."
                .into(),
        );
    }
    if !pid_alive(reg.pid) {
        xylitol_serve::remove_registration(&path);
        return Err(format!(
            "Registered Host pid {} is not running (stale registration removed).",
            reg.pid
        )
        .into());
    }
    request_exit(reg.pid)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while pid_alive(reg.pid) {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "Timed out waiting for Host pid {} to exit after SIGTERM.",
                reg.pid
            )
            .into());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    xylitol_serve::remove_registration(&path);
    eprintln!("Stopped Host pid {}", reg.pid);
    Ok(())
}

pub(crate) fn request_exit(pid: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        let pid = i32::try_from(pid).map_err(|_| format!("invalid pid {pid}"))?;
        let rc = unsafe { libc::kill(pid, libc::SIGTERM) };
        if rc != 0 {
            return Err(format!(
                "Failed to signal pid {pid}: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        Err("serve stop is not supported on this platform".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lifecycle_defaults_persistent() {
        assert_eq!(
            parse_lifecycle_flag(None).unwrap(),
            ServeLifecycle::Persistent
        );
        assert_eq!(
            parse_lifecycle_flag(Some("last-client")).unwrap(),
            ServeLifecycle::LastClient
        );
        assert!(parse_lifecycle_flag(Some("weird")).is_err());
    }

    #[test]
    fn parse_idle_accepts_units() {
        assert_eq!(
            parse_idle_flag(Some("250ms")).unwrap(),
            Duration::from_millis(250)
        );
        assert_eq!(parse_idle_flag(Some("2s")).unwrap(), Duration::from_secs(2));
        assert!(parse_idle_flag(Some("nope")).is_err());
    }

    #[tokio::test]
    async fn stop_signals_dummy_pid_and_removes_registration() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("serve.json");
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("sleep");
        let pid = child.id();
        xylitol_serve::write_registration(
            &path,
            &xylitol_serve::Registration::own("http://127.0.0.1:1".into()),
        )
        .expect("write");
        let mut own = xylitol_serve::read_registration(&path).expect("read");
        own.pid = pid;
        xylitol_serve::write_registration(&path, &own).expect("rewrite pid");

        stop_registered_host(Some(path.clone()))
            .await
            .expect("stop dummy");
        assert!(!path.exists(), "registration must be removed");
        let status = child.wait().expect("reap");
        assert!(!status.success() || !pid_alive(pid));
    }

    #[tokio::test]
    async fn stop_missing_pid_is_actionable_and_does_not_kill_self() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("serve.json");
        let mut reg = xylitol_serve::Registration::own("http://127.0.0.1:1".into());
        reg.pid = 2_147_483_647;
        xylitol_serve::write_registration(&path, &reg).expect("write");
        let err = stop_registered_host(Some(path.clone()))
            .await
            .expect_err("missing pid");
        let msg = err.to_string();
        assert!(msg.contains("not running") || msg.contains("pid"), "{msg}");
        assert!(pid_alive(std::process::id()));
    }

    #[cfg(unix)]
    #[test]
    fn request_exit_rejects_unrepresentable_pid() {
        let err = request_exit(u32::MAX).expect_err("must not signal");
        assert!(err.contains("invalid pid"), "{err}");
    }

    #[cfg(unix)]
    #[test]
    fn request_exit_terminates_a_child() {
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("sleep");
        let pid = child.id();
        request_exit(pid).expect("SIGTERM child");
        let status = child.wait().expect("reap");
        assert!(!status.success(), "SIGTERM must end the child");
    }
}
