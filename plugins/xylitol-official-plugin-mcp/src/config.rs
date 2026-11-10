use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct McpServerConfig {
    pub name: String,
    #[serde(default = "default_mcp_transport")]
    pub transport: McpTransportKind,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub scope: McpScope,
    #[serde(default)]
    pub credential: Option<String>,
    #[serde(default = "default_connect_timeout_ms")]
    pub connect_timeout_ms: u64,
    #[serde(default = "default_call_timeout_ms")]
    pub call_timeout_ms: u64,
    #[serde(default)]
    pub keep_alive_ms: u64,
    #[serde(default)]
    pub idle_ttl_ms: u64,
    #[serde(default)]
    pub custom_transport: Option<String>,
    #[serde(default)]
    pub oauth: Option<McpOAuthConfig>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct McpOAuthConfig {
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub client_metadata_url: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub redirect_uri: Option<String>,
    #[serde(default)]
    pub keychain: McpOAuthKeychain,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpOAuthKeychain {
    #[default]
    Auto,
    Off,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            transport: default_mcp_transport(),
            command: None,
            args: None,
            url: None,
            env: None,
            headers: None,
            scope: McpScope::default(),
            credential: None,
            connect_timeout_ms: default_connect_timeout_ms(),
            call_timeout_ms: default_call_timeout_ms(),
            keep_alive_ms: 0,
            idle_ttl_ms: 0,
            custom_transport: None,
            oauth: None,
        }
    }
}

fn default_connect_timeout_ms() -> u64 {
    8_000
}

fn default_call_timeout_ms() -> u64 {
    120_000
}

impl McpOAuthConfig {
    fn validate(&self, server: &McpServerConfig) -> Result<(), String> {
        if server.transport != McpTransportKind::Http {
            return Err("oauth is only valid for http transport".into());
        }
        if let Some(client_id) = self.client_id.as_deref()
            && client_id.trim().is_empty()
        {
            return Err("oauth.client_id must not be empty".into());
        }
        if self.client_id.is_some() && self.client_metadata_url.is_some() {
            return Err(
                "oauth.client_id and oauth.client_metadata_url are mutually exclusive".into(),
            );
        }
        if let Some(raw) = self.client_metadata_url.as_deref() {
            let uri: http::Uri = raw
                .trim()
                .parse()
                .map_err(|_| "oauth.client_metadata_url must be a valid URL".to_string())?;
            if uri.scheme_str() != Some("https") || uri.host().is_none() {
                return Err("oauth.client_metadata_url must be an https URL".into());
            }
            if uri.path().is_empty() || uri.path() == "/" {
                return Err("oauth.client_metadata_url must include a non-root path".into());
            }
        }
        if let Some(raw) = self.redirect_uri.as_deref() {
            let uri: http::Uri = raw
                .trim()
                .parse()
                .map_err(|_| "oauth.redirect_uri must be a valid URL".to_string())?;
            if uri.scheme_str() != Some("http") {
                return Err("oauth.redirect_uri must use http".into());
            }
            let host = uri.host().unwrap_or("");
            if !matches!(host, "127.0.0.1" | "localhost" | "[::1]" | "::1") {
                return Err(
                    "oauth.redirect_uri must use a loopback host (127.0.0.1/localhost/::1)".into(),
                );
            }
            if uri.query().is_some() {
                return Err("oauth.redirect_uri must not include a query".into());
            }
        }
        for scope in &self.scopes {
            let scope = scope.trim();
            if scope.is_empty() || scope.chars().any(char::is_whitespace) {
                return Err("oauth.scopes entries must be non-empty without whitespace".into());
            }
        }
        Ok(())
    }
}

impl McpServerConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("mcp server name must not be empty".into());
        }
        if let Some(invalid) = self
            .name
            .chars()
            .find(|c| !c.is_ascii_alphanumeric() && *c != '_' && *c != '-')
        {
            return Err(format!(
                "mcp server name '{}' is invalid: character '{invalid}' is not allowed; use [A-Za-z0-9_-] (no '/')",
                self.name
            ));
        }
        if self.connect_timeout_ms == 0 {
            return Err("connect_timeout_ms must be positive".into());
        }
        if self.call_timeout_ms == 0 {
            return Err("call_timeout_ms must be positive".into());
        }
        if let Some(oauth) = &self.oauth {
            oauth.validate(self)?;
        }
        if self
            .credential
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err("credential must not be empty".into());
        }
        if self.transport == McpTransportKind::Custom {
            let name = self
                .custom_transport
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .ok_or_else(|| "custom_transport is required for custom transport".to_string())?;
            if !is_custom_transport_registered(name) {
                return Err(format!("custom transport `{name}` is not registered"));
            }
        } else if self.custom_transport.is_some() {
            return Err("custom_transport is only valid for custom transport".into());
        }
        match self.transport {
            McpTransportKind::Stdio => match self.command.as_deref().map(str::trim) {
                None | Some("") => Err("command is required for stdio transport".into()),
                Some(_) => Ok(()),
            },
            McpTransportKind::Http => match self.url.as_deref().map(str::trim) {
                None | Some("") => Err("url is required for http transport".into()),
                Some(u) if u.ends_with("/sse") => Err(format!(
                    "legacy HTTP+SSE endpoint is not supported: {u}; use Streamable HTTP"
                )),
                Some(u) if !(u.starts_with("http://") || u.starts_with("https://")) => {
                    Err(format!("url must be http(s) for http transport: {u}"))
                }
                Some(_) => Ok(()),
            },
            McpTransportKind::Custom => Ok(()),
        }
    }

    pub fn effective_credential_id(&self) -> String {
        self.credential
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(&self.name)
            .to_string()
    }

    pub fn fingerprint(&self) -> String {
        let identity = serde_json::json!({
            "name": self.name,
            "transport": self.transport,
            "command": self.command,
            "args": self.args,
            "url": self.url,
            "env": self.env,
            "headers": self.headers,
            "oauth": self.oauth,
        });
        let bytes = serde_json::to_vec(&identity).unwrap_or_default();
        let digest = Sha256::digest(&bytes);
        digest.iter().map(|b| format!("{b:02x}")).collect()
    }
}

fn custom_transport_registry() -> &'static Mutex<HashSet<String>> {
    static REGISTRY: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashSet::new()))
}

pub fn register_custom_transport(name: impl Into<String>) {
    custom_transport_registry()
        .lock()
        .expect("custom transport registry")
        .insert(name.into());
}

pub fn is_custom_transport_registered(name: &str) -> bool {
    custom_transport_registry()
        .lock()
        .expect("custom transport registry")
        .contains(name)
}

fn default_mcp_transport() -> McpTransportKind {
    McpTransportKind::Stdio
}

#[derive(Clone, Debug, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpTransportKind {
    Stdio,
    Http,
    Custom,
}

impl<'de> Deserialize<'de> for McpTransportKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        match raw.as_str() {
            "stdio" => Ok(Self::Stdio),
            "http" => Ok(Self::Http),
            "custom" => Ok(Self::Custom),
            "sse" => Err(serde::de::Error::custom(
                "`transport: sse` is removed; use `http` (Streamable HTTP). Legacy HTTP+SSE is not supported.",
            )),
            other => Err(serde::de::Error::custom(format!(
                "unknown MCP transport `{other}`; expected `stdio` or `http`"
            ))),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum McpScope {
    #[default]
    Workspace,
    Shared,
    Session,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(name: &str, command: &str) -> McpServerConfig {
        McpServerConfig {
            name: name.into(),
            command: Some(command.into()),
            ..Default::default()
        }
    }

    fn oauth_server() -> McpServerConfig {
        McpServerConfig {
            name: "remote".into(),
            transport: McpTransportKind::Http,
            url: Some("https://mcp.example.com/mcp".into()),
            oauth: Some(McpOAuthConfig {
                client_id: Some("client-1".into()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn oauth_requires_http_transport() {
        let mut cfg = oauth_server();
        cfg.transport = McpTransportKind::Stdio;
        cfg.command = Some("bin".into());
        let err = cfg.validate().unwrap_err();
        assert!(err.contains("only valid for http"), "{err}");
    }

    #[test]
    fn oauth_rejects_client_secret_and_resource_fields() {
        for field in ["client_secret", "resource"] {
            let mut value = serde_json::to_value(oauth_server()).expect("server");
            value["oauth"][field] = serde_json::json!("x");
            let err = serde_json::from_value::<McpServerConfig>(value)
                .expect_err("unknown oauth field must be rejected");
            assert!(err.to_string().contains(field), "{err}");
        }
    }

    #[test]
    fn oauth_redirect_uri_must_be_loopback_http() {
        let mut cfg = oauth_server();
        cfg.oauth.as_mut().expect("oauth").redirect_uri =
            Some("https://evil.example/callback".into());
        assert!(cfg.validate().unwrap_err().contains("must use http"));

        cfg.oauth.as_mut().expect("oauth").redirect_uri =
            Some("http://evil.example/callback".into());
        assert!(cfg.validate().unwrap_err().contains("loopback"));

        cfg.oauth.as_mut().expect("oauth").redirect_uri =
            Some("http://127.0.0.1:43123/callback".into());
        cfg.validate().expect("loopback redirect");
    }

    #[test]
    fn oauth_client_metadata_url_requires_https_path() {
        let mut cfg = oauth_server();
        {
            let oauth = cfg.oauth.as_mut().expect("oauth");
            oauth.client_id = None;
            oauth.client_metadata_url = Some("https://example.com/client.json".into());
        }
        cfg.validate().expect("CIMD url");

        cfg.oauth.as_mut().expect("oauth").client_metadata_url =
            Some("https://example.com/".into());
        assert!(cfg.validate().unwrap_err().contains("non-root path"));
    }

    #[test]
    fn oauth_fingerprint_tracks_config() {
        let left = oauth_server();
        let mut right = left.clone();
        right.oauth.as_mut().expect("oauth").scopes = vec!["read".into()];
        assert_ne!(left.fingerprint(), right.fingerprint());
    }

    #[test]
    fn effective_credential_defaults_to_server_name() {
        let cfg = oauth_server();
        assert_eq!(cfg.effective_credential_id(), "remote");
        let mut cfg = cfg;
        cfg.credential = Some("acct-a".into());
        assert_eq!(cfg.effective_credential_id(), "acct-a");
    }

    #[test]
    fn fingerprint_ignores_scope_and_credential() {
        let mut left = server("fs", "fs-server");
        let right = McpServerConfig {
            scope: McpScope::Shared,
            credential: Some("a".into()),
            ..left.clone()
        };
        assert_eq!(left.fingerprint(), right.fingerprint());
        left.args = Some(vec!["--x".into()]);
        assert_ne!(left.fingerprint(), right.fingerprint());
    }

    #[test]
    fn validate_http_requires_url() {
        let cfg = McpServerConfig {
            name: "x".into(),
            transport: McpTransportKind::Http,
            ..Default::default()
        };
        assert!(cfg.validate().unwrap_err().contains("url"));
    }

    #[test]
    fn sse_transport_is_rejected_with_migration_hint() {
        let err = serde_json::from_value::<McpServerConfig>(serde_json::json!({
            "name": "x",
            "transport": "sse"
        }))
        .expect_err("sse must be rejected");
        let message = err.to_string();
        assert!(message.contains("sse"), "{message}");
        assert!(message.contains("http"), "{message}");
    }

    #[test]
    fn legacy_sse_url_is_rejected() {
        let cfg = McpServerConfig {
            name: "x".into(),
            transport: McpTransportKind::Http,
            url: Some("https://example.com/sse".into()),
            ..Default::default()
        };
        let message = cfg.validate().unwrap_err();
        assert!(message.contains("legacy HTTP+SSE"), "{message}");
    }

    #[test]
    fn custom_transport_requires_registration() {
        let cfg = McpServerConfig {
            name: "x".into(),
            transport: McpTransportKind::Custom,
            custom_transport: Some("ws-unregistered".into()),
            ..Default::default()
        };
        let message = cfg.validate().unwrap_err();
        assert!(message.contains("is not registered"), "{message}");

        register_custom_transport("ws-registered");
        let cfg = McpServerConfig {
            custom_transport: Some("ws-registered".into()),
            ..cfg
        };
        cfg.validate().expect("registered custom transport");
    }

    #[test]
    fn custom_transport_requires_name() {
        let cfg = McpServerConfig {
            name: "x".into(),
            transport: McpTransportKind::Custom,
            ..Default::default()
        };
        assert!(cfg.validate().unwrap_err().contains("custom_transport"));
    }

    #[test]
    fn custom_transport_name_rejected_for_core_transports() {
        let cfg = McpServerConfig {
            name: "x".into(),
            command: Some("bin".into()),
            custom_transport: Some("ws".into()),
            ..Default::default()
        };
        assert!(
            cfg.validate()
                .unwrap_err()
                .contains("only valid for custom transport")
        );
    }

    #[test]
    fn server_name_must_be_provider_safe() {
        let bad = McpServerConfig {
            name: "team/docs".into(),
            command: Some("bin".into()),
            ..Default::default()
        };
        let err = bad.validate().unwrap_err();
        assert!(err.contains("team/docs"), "{err}");
        assert!(err.contains("'/'"), "{err}");

        let good = McpServerConfig {
            name: "team-docs_1".into(),
            command: Some("bin".into()),
            ..Default::default()
        };
        good.validate().expect("provider-safe name");
    }

    #[test]
    fn timeout_defaults_and_validation() {
        let cfg = McpServerConfig::default();
        assert_eq!(cfg.connect_timeout_ms, 8_000);
        assert_eq!(cfg.call_timeout_ms, 120_000);
        let zero = McpServerConfig {
            name: "x".into(),
            command: Some("bin".into()),
            call_timeout_ms: 0,
            ..Default::default()
        };
        assert!(zero.validate().unwrap_err().contains("call_timeout_ms"));
    }
}
