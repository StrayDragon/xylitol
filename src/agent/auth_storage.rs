//! AuthStorage — OAuth credential persistence aligning with pi's auth-storage.ts.
//!
//! Stores OAuth credentials to `~/.xylitol/auth.json` with expiration tracking.
//! Supports automatic token refresh when within 5-minute expiry window.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// OAuth credentials for a provider/model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCredentials {
    pub access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Unix timestamp (seconds) when the token expires.
    pub expires_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// Auth storage for OAuth credentials.
#[derive(Debug, Clone)]
pub struct AuthStorage {
    storage_path: PathBuf,
}

impl Default for AuthStorage {
    fn default() -> Self {
        Self::new(Self::default_path())
    }
}

impl AuthStorage {
    pub fn new(storage_dir: impl Into<PathBuf>) -> Self {
        let dir = storage_dir.into();
        std::fs::create_dir_all(&dir).ok();
        Self {
            storage_path: dir.join("auth.json"),
        }
    }

    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".xylitol")
    }

    fn read_store(&self) -> HashMap<String, AuthCredentials> {
        if !self.storage_path.exists() {
            return HashMap::new();
        }
        match std::fs::read_to_string(&self.storage_path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => HashMap::new(),
        }
    }

    fn write_store(&self, store: &HashMap<String, AuthCredentials>) -> Result<(), String> {
        let json = serde_json::to_string_pretty(store)
            .map_err(|e| format!("serialize auth store: {e}"))?;
        let tmp = self.storage_path.with_extension("json.tmp");
        std::fs::write(&tmp, format!("{json}\n")).map_err(|e| format!("write auth temp: {e}"))?;
        std::fs::rename(&tmp, &self.storage_path).map_err(|e| format!("rename auth store: {e}"))?;
        Ok(())
    }

    /// Get stored credentials for a provider.
    pub fn get_auth(&self, provider_name: &str) -> Option<AuthCredentials> {
        self.read_store().get(provider_name).cloned()
    }

    /// Store credentials for a provider.
    pub fn set_auth(&self, provider_name: &str, creds: AuthCredentials) -> Result<(), String> {
        let mut store = self.read_store();
        store.insert(provider_name.to_string(), creds);
        self.write_store(&store)
    }

    /// Check if credentials are expired or expiring within the given window (seconds).
    pub fn is_expired(&self, creds: &AuthCredentials, window_secs: i64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        now >= (creds.expires_at - window_secs)
    }

    /// Check if refresh is needed (within 5-minute window).
    pub fn needs_refresh(&self, creds: &AuthCredentials) -> bool {
        self.is_expired(creds, 300)
    }

    /// Get valid credentials (refresh if needed).
    /// Note: actual refresh logic is provider-specific; this method
    /// returns None when refresh is needed but no refresh_token is available.
    pub fn get_valid_auth(&self, provider_name: &str) -> Option<AuthCredentials> {
        let creds = self.get_auth(provider_name)?;
        if !self.needs_refresh(&creds) {
            Some(creds)
        } else {
            // Token needs refresh — caller must handle this
            None
        }
    }

    /// Remove stored credentials for a provider.
    pub fn remove_auth(&self, provider_name: &str) -> Result<(), String> {
        let mut store = self.read_store();
        store.remove(provider_name);
        self.write_store(&store)
    }

    /// Check if any credentials are stored for a provider.
    pub fn has_auth(&self, provider_name: &str) -> bool {
        self.read_store().contains_key(provider_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_creds() -> AuthCredentials {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        AuthCredentials {
            access_token: "test-token".into(),
            refresh_token: Some("refresh-token".into()),
            expires_at: now + 3600, // 1 hour
            token_type: Some("Bearer".into()),
            scope: None,
        }
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let storage = AuthStorage::new(dir.path().to_path_buf());
        let creds = make_creds();

        storage.set_auth("anthropic", creds.clone()).unwrap();
        let loaded = storage.get_auth("anthropic").unwrap();
        assert_eq!(loaded.access_token, "test-token");
        assert_eq!(loaded.refresh_token.as_deref(), Some("refresh-token"));
    }

    #[test]
    fn test_no_auth_for_missing_provider() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let storage = AuthStorage::new(dir.path().to_path_buf());
        assert!(storage.get_auth("nonexistent").is_none());
    }

    #[test]
    fn test_expired_detection() {
        let storage = AuthStorage::default();
        let mut creds = make_creds();
        // Set expiry to now - 10 seconds (already expired)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        creds.expires_at = now - 10;
        assert!(storage.is_expired(&creds, 300));
    }

    #[test]
    fn test_not_expired() {
        let storage = AuthStorage::default();
        let creds = make_creds();
        assert!(!storage.is_expired(&creds, 300));
    }

    #[test]
    fn test_needs_refresh() {
        let storage = AuthStorage::default();
        let mut creds = make_creds();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        creds.expires_at = now + 60; // expires in 60 seconds
        assert!(storage.needs_refresh(&creds));
    }

    #[test]
    fn test_remove_auth() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let storage = AuthStorage::new(dir.path().to_path_buf());
        storage.set_auth("test", make_creds()).unwrap();
        assert!(storage.has_auth("test"));
        storage.remove_auth("test").unwrap();
        assert!(!storage.has_auth("test"));
    }
}
