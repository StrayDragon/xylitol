use std::collections::HashMap;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredMcpCredential {
    pub access_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
}

impl fmt::Debug for StoredMcpCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StoredMcpCredential")
            .field("access_token", &"<redacted>")
            .field(
                "refresh_token",
                &if self.refresh_token.is_some() {
                    "<redacted>"
                } else {
                    "<none>"
                },
            )
            .field("expires_at", &self.expires_at)
            .field("issuer", &self.issuer)
            .finish()
    }
}

pub struct McpCredentialStore {
    path: PathBuf,
}

impl McpCredentialStore {
    pub fn new(agent_dir: &Path) -> Self {
        Self {
            path: agent_dir.join("mcp_credentials.json"),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn read(&self) -> std::io::Result<HashMap<String, StoredMcpCredential>> {
        match std::fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(HashMap::new()),
            Err(e) => Err(e),
        }
    }

    fn write(&self, credentials: &HashMap<String, StoredMcpCredential>) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temp = self.path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(credentials)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::rename(&temp, &self.path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    pub fn get(&self, credential_id: &str) -> std::io::Result<Option<StoredMcpCredential>> {
        Ok(self.read()?.remove(credential_id))
    }

    pub fn set(
        &self,
        credential_id: impl Into<String>,
        credential: StoredMcpCredential,
    ) -> std::io::Result<()> {
        let mut credentials = self.read()?;
        credentials.insert(credential_id.into(), credential);
        self.write(&credentials)
    }

    pub fn remove(&self, credential_id: &str) -> std::io::Result<bool> {
        let mut credentials = self.read()?;
        let removed = credentials.remove(credential_id).is_some();
        if removed {
            self.write(&credentials)?;
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (McpCredentialStore, PathBuf) {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "xylitol-mcp-credentials-{}-{unique}",
            std::process::id()
        ));
        (McpCredentialStore::new(&dir), dir)
    }

    fn credential() -> StoredMcpCredential {
        StoredMcpCredential {
            access_token: "access-secret".into(),
            refresh_token: Some("refresh-secret".into()),
            expires_at: Some(42),
            issuer: Some("https://issuer.example".into()),
        }
    }

    #[test]
    fn round_trip_and_remove() {
        let (store, dir) = store();
        assert!(store.get("account-a").unwrap().is_none());
        store.set("account-a", credential()).unwrap();
        let got = store.get("account-a").unwrap().expect("stored");
        assert_eq!(got.access_token, "access-secret");
        assert!(store.remove("account-a").unwrap());
        assert!(store.get("account-a").unwrap().is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[test]
    fn file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let (store, dir) = store();
        store.set("account-a", credential()).unwrap();
        let mode = std::fs::metadata(store.path())
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn debug_redacts_tokens() {
        let text = format!("{:?}", credential());
        assert!(!text.contains("access-secret"), "{text}");
        assert!(!text.contains("refresh-secret"), "{text}");
        assert!(text.contains("redacted"), "{text}");
    }
}
