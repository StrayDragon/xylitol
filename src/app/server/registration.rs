//! Serve registration file (c2475): discovery contract for the daemon.
//!
//! `~/.xylitol/serve.json` (0600, atomic tmp+rename write). The daemon re-reads
//! it on a fixed interval and reports eviction when the contents no longer
//! match (replaced by a newer daemon or deleted). Attach-side readers use it
//! to produce actionable failure diagnostics.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::infra::resource::DefaultResourceLoader;

pub const REGISTRATION_FILE_NAME: &str = "serve.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registration {
    pub url: String,
    pub pid: u32,
    pub version: String,
}

impl Registration {
    /// Registration describing this process.
    pub fn own(url: String) -> Self {
        Self {
            url,
            pid: std::process::id(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Default registration path: `~/.xylitol/serve.json` (data dir convention).
pub fn default_registration_path() -> PathBuf {
    DefaultResourceLoader::default_agent_dir().join(REGISTRATION_FILE_NAME)
}

/// Atomic write (tmp + rename) with 0600 permissions on the temp file.
pub fn write_registration(path: &Path, reg: &Registration) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(reg).map_err(std::io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    let _ = std::fs::remove_file(&tmp);
    {
        use std::io::Write;
        let mut file = {
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                std::fs::OpenOptions::new()
                    .mode(0o600)
                    .write(true)
                    .create_new(true)
                    .open(&tmp)?
            }
            #[cfg(not(unix))]
            {
                std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&tmp)?
            }
        };
        file.write_all(body.as_bytes())?;
        file.flush()?;
    }
    std::fs::rename(&tmp, path)
}

pub fn read_registration(path: &Path) -> std::io::Result<Registration> {
    let body = std::fs::read_to_string(path)?;
    serde_json::from_str(&body).map_err(std::io::Error::other)
}

pub fn remove_registration(path: &Path) {
    let _ = std::fs::remove_file(path);
}

/// Self-eviction loop (c2475 sr-reg1): poll the registration file; when it no
/// longer matches `own` (replaced or deleted), invoke `on_evicted` once and
/// stop. The caller owns the shutdown/exit decision.
pub async fn run_self_check(
    path: PathBuf,
    own: Registration,
    interval: Duration,
    on_evicted: impl FnOnce(),
) {
    loop {
        tokio::time::sleep(interval).await;
        let evicted = match read_registration(&path) {
            Ok(current) => current != own,
            Err(_) => true,
        };
        if evicted {
            on_evicted();
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        std::env::temp_dir().join(format!("xylitol-reg-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn write_read_roundtrip_is_atomic_0600() {
        let dir = temp_dir();
        let path = dir.join("serve.json");
        let reg = Registration::own("http://127.0.0.1:18790".into());
        write_registration(&path, &reg).expect("write");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).expect("meta").permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "registration must be 0600");
        }
        assert_eq!(read_registration(&path).expect("read"), reg);
        assert!(!dir.join("serve.json.tmp").exists(), "tmp must be renamed");
        remove_registration(&path);
        assert!(!path.exists());
        assert!(read_registration(&path).is_err());
    }

    #[test]
    fn read_missing_is_err() {
        assert!(read_registration(&temp_dir().join("serve.json")).is_err());
    }

    #[tokio::test]
    async fn self_check_fires_on_takeover() {
        let dir = temp_dir();
        let path = dir.join("serve.json");
        let own = Registration::own("http://127.0.0.1:1".into());
        write_registration(&path, &own).expect("write");

        let (tx, mut rx) = tokio::sync::watch::channel(false);
        tokio::spawn(run_self_check(
            path.clone(),
            own.clone(),
            Duration::from_millis(20),
            move || {
                let _ = tx.send(true);
            },
        ));
        // 旧 daemon 仍在轮询；新 daemon 顶替注册文件。
        tokio::time::sleep(Duration::from_millis(60)).await;
        let newcomer = Registration {
            pid: own.pid + 1,
            ..own
        };
        write_registration(&path, &newcomer).expect("takeover write");
        rx.changed().await.expect("eviction signal");
        remove_registration(&path);
    }

    #[tokio::test]
    async fn self_check_fires_on_delete() {
        let dir = temp_dir();
        let path = dir.join("serve.json");
        let own = Registration::own("http://127.0.0.1:2".into());
        write_registration(&path, &own).expect("write");

        let (tx, mut rx) = tokio::sync::watch::channel(false);
        tokio::spawn(run_self_check(
            path.clone(),
            own,
            Duration::from_millis(20),
            move || {
                let _ = tx.send(true);
            },
        ));
        tokio::time::sleep(Duration::from_millis(60)).await;
        remove_registration(&path);
        rx.changed().await.expect("eviction signal");
    }
}
