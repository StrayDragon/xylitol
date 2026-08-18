//! Model manifest loader — loads model metadata from JSON/YAML configuration.
//!
//! Allows users to define custom providers and models without code changes.
//! Format:
//! ```json
//! {
//!   "models": [
//!     {
//!       "id": "my-custom-model",
//!       "provider": "openai",
//!       "api": "openai-responses",
//!       "display_name": "My Custom Model",
//!       "context_window": 128000,
//!       "thinking": true,
//!       "base_url": "https://custom-endpoint.com/v1"
//!     }
//!   ]
//! }
//! ```
//! Omitting `api` uses [`XyModelKind::default_adapter_api`]
//! (OpenAI → `openai-responses`, Anthropic → `anthropic-messages`).

use std::path::Path;

use serde::Deserialize;

use super::registry::ModelRegistry;
use crate::protocol::model::XyModelMeta;
use crate::protocol::model::{XyModelConfig, XyModelKind};

/// Manifest load failures (crate-private; not `Xy*`).
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("read manifest {path:?}: {source}")]
    Read {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("parse manifest: {0}")]
    Parse(#[from] serde_json::Error),
}

/// A single model definition from a manifest file.
#[derive(Debug, Clone, Deserialize)]
pub struct ManifestModel {
    pub id: String,
    pub provider: String,
    /// Adapter dialect; omitted → [`XyModelKind::default_adapter_api`] for provider (c1600).
    #[serde(default)]
    pub api: Option<String>,
    pub display_name: Option<String>,
    #[serde(default = "default_context_window")]
    pub context_window: u64,
    #[serde(default)]
    pub thinking: bool,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub cost_input: f64,
    #[serde(default)]
    pub cost_output: f64,
    #[serde(default)]
    pub cost_cache_read: f64,
    #[serde(default)]
    pub cost_cache_write: f64,
    #[serde(default)]
    pub max_tokens: u64,
    #[serde(default)]
    pub thinking_levels: Vec<String>,
}

fn default_context_window() -> u64 {
    128_000
}

/// A complete model manifest file.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelManifest {
    pub models: Vec<ManifestModel>,
}

/// Load models from a JSON manifest file and register them into a
/// [`ModelRegistry`].
///
/// The manifest file follows the format:
/// ```json
/// { "models": [ { "id": "...", "provider": "...", ... } ] }
/// ```
pub fn load_models_from_manifest(
    path: &Path,
    registry: &mut ModelRegistry,
    default_api_key: Option<&str>,
) -> Result<usize, ManifestError> {
    let content = std::fs::read_to_string(path).map_err(|source| ManifestError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    let manifest: ModelManifest = serde_json::from_str(&content)?;

    let mut count = 0;
    for m in &manifest.models {
        let kind = match m.provider.as_str() {
            "openai" => XyModelKind::OpenAi,
            "anthropic" => XyModelKind::Anthropic,
            "fake" => XyModelKind::Fake,
            // Default to OpenAI-compatible for unknown providers
            _ => XyModelKind::OpenAi,
        };

        let api_key = m
            .api_key
            .clone()
            .or_else(|| default_api_key.map(String::from))
            .unwrap_or_default();

        let api = m
            .api
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| kind.default_adapter_api().to_string());

        let meta = XyModelMeta {
            id: m.id.clone(),
            config: XyModelConfig {
                kind,
                api_key,
                model: m.id.clone(),
                base_url: m.base_url.clone(),
                // Keep in sync with meta.api (c1598 / adapter resolve uses config.api).
                api: Some(api.clone()),
                compat: None,
            },
            display_name: m.display_name.clone().unwrap_or_else(|| m.id.clone()),
            thinking: m.thinking,
            context_window: m.context_window,
            api,
            provider: m.provider.clone(),
            cost_input: m.cost_input,
            cost_output: m.cost_output,
            cost_cache_read: m.cost_cache_read,
            cost_cache_write: m.cost_cache_write,
            max_tokens: m.max_tokens,
            thinking_levels: m.thinking_levels.clone(),
            thinking_level_map: Default::default(),
        };

        registry.register(meta);
        count += 1;
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::NamedTempFile;

    fn empty_registry() -> ModelRegistry {
        ModelRegistry::new(Arc::new(
            crate::infra::config::value::InfraSecretResolver::new(),
        ))
    }

    #[test]
    fn test_load_basic_manifest() {
        let json = r#"
        {
            "models": [
                {
                    "id": "gpt-4o",
                    "provider": "openai",
                    "display_name": "GPT-4o",
                    "context_window": 128000,
                    "thinking": true
                },
                {
                    "id": "claude-opus-4",
                    "provider": "anthropic",
                    "context_window": 200000,
                    "thinking": true
                }
            ]
        }
        "#;

        let file = NamedTempFile::new().unwrap();
        std::fs::write(file.path(), json).unwrap();

        let mut reg = empty_registry();
        let count = load_models_from_manifest(file.path(), &mut reg, None).unwrap();
        assert_eq!(count, 2);
        assert_eq!(reg.len(), 2);

        let gpt = reg.find("gpt-4o").unwrap();
        assert_eq!(gpt.config.provider_name(), "openai");
        assert_eq!(gpt.context_window, 128000);
        assert!(gpt.thinking);
        assert_eq!(gpt.api, "openai-responses");
        assert_eq!(gpt.config.api.as_deref(), Some("openai-responses"));

        let claude = reg.find("claude-opus-4").unwrap();
        assert_eq!(claude.config.provider_name(), "anthropic");
        assert_eq!(claude.api, "anthropic-messages");
        assert_eq!(claude.config.api.as_deref(), Some("anthropic-messages"));
    }

    #[test]
    fn manifest_completions_api_selects_completions_adapter() {
        let json = r#"
        {
            "models": [
                {
                    "id": "zen-free",
                    "provider": "openai",
                    "api": "openai-completions"
                }
            ]
        }
        "#;
        let file = NamedTempFile::new().unwrap();
        std::fs::write(file.path(), json).unwrap();
        let mut reg = empty_registry();
        load_models_from_manifest(file.path(), &mut reg, None).unwrap();
        let m = reg.find("zen-free").unwrap();
        assert_eq!(m.api, "openai-completions");
        assert_eq!(m.config.api.as_deref(), Some("openai-completions"));
        assert_eq!(
            crate::infra::provider::adapter::factory::resolve_adapter_kind(&m.config),
            crate::infra::provider::adapter::AdapterKind::OpenAiCompletions
        );
    }

    #[test]
    fn test_load_manifest_with_custom_base_url() {
        let json = r#"
        {
            "models": [
                {
                    "id": "my-model",
                    "provider": "openai",
                    "base_url": "https://custom.example.com/v1",
                    "context_window": 64000
                }
            ]
        }
        "#;

        let file = NamedTempFile::new().unwrap();
        std::fs::write(file.path(), json).unwrap();

        let mut reg = empty_registry();
        let count = load_models_from_manifest(file.path(), &mut reg, Some("sk-default")).unwrap();
        assert_eq!(count, 1);

        let model = reg.find("my-model").unwrap();
        assert_eq!(
            model.config.base_url.as_deref(),
            Some("https://custom.example.com/v1")
        );
        assert_eq!(model.config.api_key, "sk-default");
    }

    #[test]
    fn test_load_empty_manifest() {
        let json = r#"{"models": []}"#;
        let file = NamedTempFile::new().unwrap();
        std::fs::write(file.path(), json).unwrap();

        let mut reg = empty_registry();
        let count = load_models_from_manifest(file.path(), &mut reg, None).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let mut reg = empty_registry();
        let result = load_models_from_manifest(Path::new("/nonexistent/file.json"), &mut reg, None);
        assert!(result.is_err());
    }
}
