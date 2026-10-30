use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::credential::write_private_file;

pub const REGISTRATION_FILE_NAME: &str = "serve.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registration {
    pub url: String,
    pub pid: u32,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

impl Registration {
    pub fn own(url: String) -> Self {
        Self::own_with_token(url, None)
    }

    pub fn own_with_token(url: String, token: Option<String>) -> Self {
        Self {
            url,
            pid: std::process::id(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            token,
        }
    }
}

pub fn default_registration_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".xylitol")
        .join(REGISTRATION_FILE_NAME)
}

pub fn write_registration(path: &Path, reg: &Registration) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(reg).map_err(std::io::Error::other)?;
    write_private_file(path, &body)
}

pub fn read_registration(path: &Path) -> std::io::Result<Registration> {
    let body = std::fs::read_to_string(path)?;
    serde_json::from_str(&body).map_err(std::io::Error::other)
}

pub fn remove_registration(path: &Path) {
    let _ = std::fs::remove_file(path);
}

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
        assert!(!dir.join("serve.tmp").exists(), "tmp must be renamed");
        remove_registration(&path);
        assert!(!path.exists());
        assert!(read_registration(&path).is_err());
    }

    #[test]
    fn token_roundtrip_and_none_is_omitted() {
        let dir = temp_dir();
        let path = dir.join("serve.json");
        let with_token = Registration::own_with_token(
            "http://127.0.0.1:18790".into(),
            Some("secret-token".into()),
        );
        write_registration(&path, &with_token).expect("write");
        assert_eq!(read_registration(&path).expect("read"), with_token);
        let body = std::fs::read_to_string(&path).expect("body");
        assert!(body.contains("\"token\": \"secret-token\""), "{body}");

        let without = Registration::own("http://127.0.0.1:18790".into());
        write_registration(&path, &without).expect("write");
        let body = std::fs::read_to_string(&path).expect("body");
        assert!(
            !body.contains("\"token\""),
            "None token must be omitted: {body}"
        );
        let _ = std::fs::remove_dir_all(&dir);
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
