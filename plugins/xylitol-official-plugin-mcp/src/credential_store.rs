use std::collections::HashMap;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use oauth2::TokenResponse;
use rmcp::transport::auth::{AuthError, CredentialStore, StoredCredentials};
use serde::{Deserialize, Serialize};

use crate::config::McpOAuthKeychain;

const KEYCHAIN_SERVICE: &str = "xylitol-mcp";

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredMcpCredential {
    pub access_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(default)]
    pub granted_scopes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_received_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_response: Option<serde_json::Value>,
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
            .field("client_id", &self.client_id)
            .field("granted_scopes", &self.granted_scopes)
            .field("token_received_at", &self.token_received_at)
            .field("token_type", &self.token_type)
            .field(
                "token_response",
                &if self.token_response.is_some() {
                    "<redacted>"
                } else {
                    "<none>"
                },
            )
            .finish()
    }
}

impl StoredMcpCredential {
    fn to_rmcp(&self) -> Result<StoredCredentials, AuthError> {
        let client_id = self
            .client_id
            .clone()
            .filter(|id| !id.trim().is_empty())
            .ok_or_else(|| {
                AuthError::CredentialStoreError(
                    "stored OAuth credential is missing client_id".into(),
                )
            })?;
        let token_response = match self.token_response.as_ref() {
            Some(value) => Some(serde_json::from_value(value.clone()).map_err(|error| {
                AuthError::CredentialStoreError(format!(
                    "stored OAuth token response is invalid: {error}"
                ))
            })?),
            None => {
                let mut value = serde_json::json!({
                    "access_token": self.access_token,
                });
                if let Some(refresh_token) = self.refresh_token.as_ref() {
                    value["refresh_token"] = serde_json::json!(refresh_token);
                }
                if let (Some(expires_at), Some(received_at)) =
                    (self.expires_at, self.token_received_at)
                {
                    value["expires_in"] = serde_json::json!(expires_at.saturating_sub(received_at));
                }
                if let Some(token_type) = self.token_type.as_ref() {
                    value["token_type"] = serde_json::json!(token_type);
                }
                if !self.granted_scopes.is_empty() {
                    value["scope"] = serde_json::json!(self.granted_scopes.join(" "));
                }
                Some(serde_json::from_value(value).map_err(|error| {
                    AuthError::CredentialStoreError(format!(
                        "stored OAuth token response is invalid: {error}"
                    ))
                })?)
            }
        };
        Ok(StoredCredentials::new(
            client_id,
            token_response,
            self.granted_scopes.clone(),
            self.token_received_at,
        )
        .with_issuer(self.issuer.clone()))
    }

    fn from_rmcp(credentials: StoredCredentials) -> Result<Self, AuthError> {
        let token_response = credentials.token_response.as_ref().ok_or_else(|| {
            AuthError::CredentialStoreError(
                "OAuth credentials have no token response to store".into(),
            )
        })?;
        let token_response_json = serde_json::to_value(token_response).map_err(|error| {
            AuthError::CredentialStoreError(format!(
                "failed to serialize OAuth token response: {error}"
            ))
        })?;
        let access_token = token_response.access_token().secret().to_string();
        let refresh_token = token_response
            .refresh_token()
            .map(|token| token.secret().to_string());
        let expires_in = token_response
            .expires_in()
            .map(|duration| duration.as_secs());
        let expires_at = credentials
            .token_received_at
            .zip(expires_in)
            .map(|(received_at, expires_in)| received_at.saturating_add(expires_in));
        let token_type = format!("{:?}", token_response.token_type());
        Ok(Self {
            access_token,
            refresh_token,
            expires_at,
            issuer: credentials.issuer,
            client_id: Some(credentials.client_id),
            granted_scopes: credentials.granted_scopes,
            token_received_at: credentials.token_received_at,
            token_type: Some(token_type),
            token_response: Some(token_response_json),
        })
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

pub struct OAuthCredentialStore {
    credential_id: String,
    keychain: McpOAuthKeychain,
    file: McpCredentialStore,
}

impl OAuthCredentialStore {
    pub fn new(
        agent_dir: &Path,
        credential_id: impl Into<String>,
        keychain: McpOAuthKeychain,
    ) -> Self {
        Self {
            credential_id: credential_id.into(),
            keychain,
            file: McpCredentialStore::new(agent_dir),
        }
    }

    pub fn file_store(&self) -> &McpCredentialStore {
        &self.file
    }

    async fn keychain_load(&self) -> Result<Option<StoredMcpCredential>, AuthError> {
        let credential_id = self.credential_id.clone();
        let raw = tokio::task::spawn_blocking(move || {
            let entry = keyring::Entry::new(KEYCHAIN_SERVICE, &credential_id)
                .map_err(|error| error.to_string())?;
            entry.get_password().map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| AuthError::CredentialStoreError(error.to_string()))?;
        match raw {
            Ok(value) => serde_json::from_str(&value).map(Some).map_err(|error| {
                AuthError::CredentialStoreError(format!("keychain credential is invalid: {error}"))
            }),
            Err(_) => Ok(None),
        }
    }

    async fn keychain_save(&self, credential: &StoredMcpCredential) -> Result<(), AuthError> {
        let credential_id = self.credential_id.clone();
        let raw = serde_json::to_string(credential).map_err(|error| {
            AuthError::CredentialStoreError(format!(
                "failed to serialize keychain credential: {error}"
            ))
        })?;
        tokio::task::spawn_blocking(move || {
            let entry = keyring::Entry::new(KEYCHAIN_SERVICE, &credential_id)
                .map_err(|error| error.to_string())?;
            entry.set_password(&raw).map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| AuthError::CredentialStoreError(error.to_string()))?
        .map_err(AuthError::CredentialStoreError)
    }

    async fn keychain_clear(&self) -> Result<(), AuthError> {
        let credential_id = self.credential_id.clone();
        tokio::task::spawn_blocking(move || {
            let entry = keyring::Entry::new(KEYCHAIN_SERVICE, &credential_id)
                .map_err(|error| error.to_string())?;
            entry.delete_credential().map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| AuthError::CredentialStoreError(error.to_string()))?
        .map_err(AuthError::CredentialStoreError)
    }
}

#[async_trait]
impl CredentialStore for OAuthCredentialStore {
    async fn load(&self) -> Result<Option<StoredCredentials>, AuthError> {
        if self.keychain == McpOAuthKeychain::Auto {
            match self.keychain_load().await {
                Ok(Some(credential)) => return credential.to_rmcp().map(Some),
                Ok(None) => {}
                Err(error) => {
                    log::warn!(
                        "MCP OAuth keychain unavailable for {}: {error}; falling back to file",
                        self.credential_id
                    );
                }
            }
        }
        match self.file.get(&self.credential_id) {
            Ok(Some(credential)) => credential.to_rmcp().map(Some),
            Ok(None) => Ok(None),
            Err(error) => Err(AuthError::CredentialStoreError(error.to_string())),
        }
    }

    async fn save(&self, credentials: StoredCredentials) -> Result<(), AuthError> {
        let stored = StoredMcpCredential::from_rmcp(credentials)?;
        if self.keychain == McpOAuthKeychain::Auto {
            match self.keychain_save(&stored).await {
                Ok(()) => return Ok(()),
                Err(error) => {
                    log::warn!(
                        "MCP OAuth keychain unavailable for {}: {error}; falling back to file",
                        self.credential_id
                    );
                }
            }
        }
        self.file
            .set(self.credential_id.clone(), stored)
            .map_err(|error| AuthError::CredentialStoreError(error.to_string()))
    }

    async fn clear(&self) -> Result<(), AuthError> {
        if self.keychain == McpOAuthKeychain::Auto {
            let _ = self.keychain_clear().await;
        }
        self.file
            .remove(&self.credential_id)
            .map(|_| ())
            .map_err(|error| AuthError::CredentialStoreError(error.to_string()))
    }

    async fn on_refresh_token_rejected(
        &self,
        _credentials: &StoredCredentials,
    ) -> Result<(), AuthError> {
        self.clear().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::transport::auth::OAuthTokenResponse;

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
            client_id: Some("client-1".into()),
            granted_scopes: vec!["read".into()],
            token_received_at: Some(10),
            token_type: Some("Bearer".into()),
            token_response: None,
        }
    }

    fn rmcp_credential() -> StoredCredentials {
        let token: OAuthTokenResponse = serde_json::from_value(serde_json::json!({
            "access_token": "access-secret",
            "refresh_token": "refresh-secret",
            "token_type": "Bearer",
            "expires_in": 32,
            "scope": "read"
        }))
        .expect("token response");
        StoredCredentials::new(
            "client-1".into(),
            Some(token),
            vec!["read".into()],
            Some(10),
        )
        .with_issuer(Some("https://issuer.example".into()))
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

    #[test]
    fn rmcp_round_trip_keeps_metadata() {
        let stored = StoredMcpCredential::from_rmcp(rmcp_credential()).expect("store");
        assert_eq!(stored.client_id.as_deref(), Some("client-1"));
        assert_eq!(stored.granted_scopes, ["read"]);
        assert_eq!(stored.token_received_at, Some(10));
        assert_eq!(stored.token_type.as_deref(), Some("Bearer"));
        let back = stored.to_rmcp().expect("load");
        assert_eq!(back.client_id, "client-1");
        assert_eq!(back.granted_scopes, ["read"]);
        assert_eq!(back.token_received_at, Some(10));
        assert_eq!(back.issuer.as_deref(), Some("https://issuer.example"));
    }

    #[tokio::test]
    async fn oauth_store_file_fallback_and_refresh_rejected_clear() {
        let (_, dir) = store();
        let store = OAuthCredentialStore::new(&dir, "account-a", McpOAuthKeychain::Off);
        store.save(rmcp_credential()).await.expect("save");
        let loaded = store.load().await.expect("load").expect("credential");
        assert_eq!(loaded.client_id, "client-1");
        assert!(store.file_store().path().is_file());
        store
            .on_refresh_token_rejected(&loaded)
            .await
            .expect("clear rejected");
        assert!(store.load().await.expect("load").is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
