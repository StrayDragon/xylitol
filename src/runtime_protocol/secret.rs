//! Runtime boundary for secret/config-value resolution.

use std::collections::HashMap;

/// Secret/config-value resolver port — abstracts env-var interpolation,
/// shell-command resolution, and header resolution.
pub trait SecretResolver: Send + Sync + std::fmt::Debug {
    /// Resolve a config value to its actual string value.
    fn resolve_config_value(
        &self,
        config: &str,
        env: Option<&HashMap<String, String>>,
    ) -> Option<String>;

    /// Resolve all values in a header map using the same resolution logic.
    fn resolve_headers(
        &self,
        headers: &HashMap<String, String>,
        env: Option<&HashMap<String, String>>,
    ) -> Option<HashMap<String, String>>;
}
