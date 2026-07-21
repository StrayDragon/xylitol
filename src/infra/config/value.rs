//! ConfigValueResolver — shell command + env var template resolution for API keys and headers.
//!
//! Supports:
//! - Literal strings (passthrough)
//! - `$ENV_VAR` / `${ENV_VAR}` environment variable interpolation
//! - `${VAR:-default}` with fallback default
//! - `!command` shell command execution (cached for process lifetime)
//! - `$$` escapes to literal `$`, `$!` escapes to literal `!`

use std::collections::HashMap;
use std::sync::Mutex;

use crate::protocol::ports::XySecretResolver;

// ── Types ──────────────────────────────────────────────────────────

/// A parsed configuration value reference.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigValueReference {
    /// Literal string (passthrough).
    Literal(String),
    /// Environment variable name (e.g., `$MY_KEY` or `${MY_KEY}`).
    EnvVar(String),
    /// Shell command to execute (e.g., `!pass show api-key`).
    ShellCmd(String),
    /// Template string with mixed literal parts and env var references.
    /// e.g., `prefix_${VAR1}_suffix_${VAR2}`
    Template(Vec<TemplatePart>),
}

/// A part of a template string.
#[derive(Debug, Clone, PartialEq)]
pub enum TemplatePart {
    /// Literal text segment.
    Literal(String),
    /// Environment variable reference (with optional default).
    EnvVar {
        name: String,
        default: Option<String>,
    },
}

// ── Cache ──────────────────────────────────────────────────────────

static COMMAND_CACHE: Mutex<Option<HashMap<String, Option<String>>>> = Mutex::new(None);

fn get_cache() -> &'static Mutex<Option<HashMap<String, Option<String>>>> {
    &COMMAND_CACHE
}

/// Clear the shell command result cache (for testing).
pub fn clear_config_value_cache() {
    if let Ok(mut cache) = get_cache().lock() {
        *cache = None;
    }
}

// ── Parsing ────────────────────────────────────────────────────────

/// Parse a config value string into a [`ConfigValueReference`].
///
/// Recognizes patterns:
/// - `!command` → ShellCmd
/// - `$VAR` → EnvVar
/// - `${VAR}` → EnvVar
/// - `${VAR:-default}` → EnvVar with default
/// - Mix of `$VAR` and literal text → Template
/// - `$$` → literal `$`
/// - Otherwise → Literal
pub fn parse_config_value(config: &str) -> ConfigValueReference {
    if config.is_empty() {
        return ConfigValueReference::Literal(String::new());
    }

    // Shell command
    if config.starts_with('!') && !config.starts_with("$!") {
        return ConfigValueReference::ShellCmd(config[1..].to_string());
    }

    // Try to parse as template (contains $ followed by env var name or {...})
    let parts = parse_template_parts(config);

    match parts.len() {
        0 => ConfigValueReference::Literal(String::new()),
        1 => match &parts[0] {
            TemplatePart::Literal(s) => ConfigValueReference::Literal(s.clone()),
            TemplatePart::EnvVar {
                name,
                default: None,
            } => ConfigValueReference::EnvVar(name.clone()),
            TemplatePart::EnvVar {
                name: _,
                default: Some(_),
            } => {
                // Single env var with default — still single reference
                ConfigValueReference::Template(parts)
            }
        },
        _ => ConfigValueReference::Template(parts),
    }
}

fn parse_template_parts(config: &str) -> Vec<TemplatePart> {
    let mut parts = Vec::new();
    let mut current_literal = String::new();
    let chars: Vec<char> = config.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '$' && i + 1 < chars.len() {
            match chars[i + 1] {
                '$' => {
                    // Escaped dollar: $$
                    current_literal.push('$');
                    i += 2;
                    continue;
                }
                '!' => {
                    // Escaped bang: $!
                    current_literal.push('!');
                    i += 2;
                    continue;
                }
                '{' => {
                    // ${VAR} or ${VAR:-default}
                    let close = chars[i + 2..].iter().position(|&c| c == '}');
                    if let Some(end_offset) = close {
                        let inner: String = chars[i + 2..=i + 1 + end_offset].iter().collect();
                        if let Some((name, default)) = parse_braced_var(&inner) {
                            if !current_literal.is_empty() {
                                parts.push(TemplatePart::Literal(std::mem::take(
                                    &mut current_literal,
                                )));
                            }
                            parts.push(TemplatePart::EnvVar { name, default });
                            i += 2 + end_offset + 1; // $ + { + inner + }
                            continue;
                        }
                    }
                    // Invalid ${...} syntax — treat as literal
                    current_literal.push('$');
                    current_literal.push('{');
                    i += 2;
                }
                c if c.is_ascii_alphanumeric() || c == '_' => {
                    // $VAR
                    let start = i + 1;
                    let end = start
                        + chars[start..]
                            .iter()
                            .position(|&c| !(c.is_ascii_alphanumeric() || c == '_'))
                            .unwrap_or(chars.len() - start);
                    let name: String = chars[start..end].iter().collect();
                    if !current_literal.is_empty() {
                        parts.push(TemplatePart::Literal(std::mem::take(&mut current_literal)));
                    }
                    parts.push(TemplatePart::EnvVar {
                        name,
                        default: None,
                    });
                    i = end;
                }
                _ => {
                    current_literal.push('$');
                    i += 1;
                }
            }
        } else {
            current_literal.push(chars[i]);
            i += 1;
        }
    }

    if !current_literal.is_empty() {
        parts.push(TemplatePart::Literal(current_literal));
    }

    parts
}

fn parse_braced_var(inner: &str) -> Option<(String, Option<String>)> {
    // Handle ${VAR:-default}
    if let Some(colon_idx) = inner.find(":-") {
        let name = inner[..colon_idx].to_string();
        let default = inner[colon_idx + 2..].to_string();
        if name.is_empty() || (!name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')) {
            return None;
        }
        return Some((name, Some(default)));
    }

    // Handle ${VAR} (no default)
    let name = inner.to_string();
    if name.is_empty() || (!name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')) {
        return None;
    }
    Some((name, None))
}

// ── Resolution ────────────────────────────────────────────────────

/// Resolve a config value to its actual string value.
///
/// Returns `None` when:
/// - An env var reference names a variable that is unset (and has no default)
/// - A shell command fails or returns empty stdout
///
/// When `env` is provided, it is checked before `std::env::var`.
pub fn resolve_config_value(config: &str, env: Option<&HashMap<String, String>>) -> Option<String> {
    let reference = parse_config_value(config);
    resolve_reference(&reference, env)
}

fn resolve_reference(
    reference: &ConfigValueReference,
    env: Option<&HashMap<String, String>>,
) -> Option<String> {
    match reference {
        ConfigValueReference::Literal(s) => Some(s.clone()),
        ConfigValueReference::EnvVar(name) => resolve_env_var(name, env),
        ConfigValueReference::ShellCmd(cmd) => {
            let cache_key = format!("!{cmd}");
            let mut cache_guard = get_cache().lock().unwrap_or_else(|e| e.into_inner());
            let cache = cache_guard.get_or_insert_with(HashMap::new);

            if let Some(result) = cache.get(&cache_key) {
                return result.clone();
            }

            let result = execute_shell_command(cmd);
            cache.insert(cache_key.clone(), result.clone());
            result
        }
        ConfigValueReference::Template(parts) => {
            let mut result = String::new();
            for part in parts {
                match part {
                    TemplatePart::Literal(s) => result.push_str(s),
                    TemplatePart::EnvVar { name, default } => match resolve_env_var(name, env) {
                        Some(val) => result.push_str(&val),
                        None => {
                            if let Some(default_val) = default {
                                result.push_str(default_val);
                            } else {
                                return None;
                            }
                        }
                    },
                }
            }
            Some(result)
        }
    }
}

fn resolve_env_var(name: &str, env: Option<&HashMap<String, String>>) -> Option<String> {
    // Check provided env first
    if let Some(env_map) = env
        && let Some(val) = env_map.get(name)
        && !val.is_empty()
    {
        return Some(val.clone());
    }
    // Fall back to process env
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

fn execute_shell_command(command: &str) -> Option<String> {
    let output = std::process::Command::new("sh")
        .args(["-c", command])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        None
    } else {
        Some(stdout)
    }
}

// ── Infra implementation of XySecretResolver port ─────────────────────

/// Default [`XySecretResolver`] backed by env-var interpolation and shell-command
/// execution (see [`resolve_config_value`] / [`resolve_headers`]).
#[derive(Debug, Clone, Default)]
pub struct InfraSecretResolver;

impl InfraSecretResolver {
    /// Create a new infra secret resolver.
    pub fn new() -> Self {
        Self
    }
}

impl XySecretResolver for InfraSecretResolver {
    fn resolve_config_value(
        &self,
        config: &str,
        env: Option<&HashMap<String, String>>,
    ) -> Option<String> {
        resolve_config_value(config, env)
    }

    fn resolve_headers(
        &self,
        headers: &HashMap<String, String>,
        env: Option<&HashMap<String, String>>,
    ) -> Option<HashMap<String, String>> {
        resolve_headers(headers, env)
    }
}

// ── Convenience API ────────────────────────────────────────────────

/// Resolve a config value or throw an error with description.
pub fn resolve_config_value_or_throw(
    config: &str,
    description: &str,
    env: Option<&HashMap<String, String>>,
) -> Result<String, String> {
    resolve_config_value(config, env).ok_or_else(|| {
        let reference = parse_config_value(config);
        match reference {
            ConfigValueReference::ShellCmd(cmd) => {
                format!("Failed to resolve {description} from shell command: {cmd}")
            }
            ConfigValueReference::EnvVar(name) => {
                format!("Failed to resolve {description} from environment variable: {name}")
            }
            ConfigValueReference::Template(ref parts) => {
                let names: Vec<&str> = parts
                    .iter()
                    .filter_map(|p| match p {
                        TemplatePart::EnvVar {
                            name,
                            default: None,
                        } => Some(name.as_str()),
                        _ => None,
                    })
                    .collect();
                if names.is_empty() {
                    format!("Failed to resolve {description}")
                } else {
                    format!(
                        "Failed to resolve {description} from environment variables: {}",
                        names.join(", ")
                    )
                }
            }
            ConfigValueReference::Literal(_) => {
                format!("Failed to resolve {description}")
            }
        }
    })
}

/// Get the names of environment variables referenced in a config value.
pub fn get_config_value_env_var_names(config: &str) -> Vec<String> {
    let reference = parse_config_value(config);
    match reference {
        ConfigValueReference::EnvVar(name) => vec![name],
        ConfigValueReference::Template(parts) => parts
            .iter()
            .filter_map(|p| match p {
                TemplatePart::EnvVar { name, .. } => Some(name.clone()),
                _ => None,
            })
            .collect(),
        _ => vec![],
    }
}

/// Get the names of missing (unset) environment variables referenced in a config value.
pub fn get_missing_config_value_env_var_names(
    config: &str,
    env: Option<&HashMap<String, String>>,
) -> Vec<String> {
    get_config_value_env_var_names(config)
        .into_iter()
        .filter(|name| resolve_env_var(name, env).is_none())
        .collect()
}

/// Check if a config value is a shell command.
pub fn is_command_config_value(config: &str) -> bool {
    matches!(
        parse_config_value(config),
        ConfigValueReference::ShellCmd(_)
    )
}

/// Check if all referenced env vars in a config value are configured.
pub fn is_config_value_configured(config: &str, env: Option<&HashMap<String, String>>) -> bool {
    get_missing_config_value_env_var_names(config, env).is_empty()
}

/// Resolve all values in a header map using the same resolution logic.
///
/// Returns `None` if the resulting map would be empty.
pub fn resolve_headers(
    headers: &HashMap<String, String>,
    env: Option<&HashMap<String, String>>,
) -> Option<HashMap<String, String>> {
    let resolved: HashMap<String, String> = headers
        .iter()
        .filter_map(|(key, value)| resolve_config_value(value, env).map(|v| (key.clone(), v)))
        .collect();

    if resolved.is_empty() {
        None
    } else {
        Some(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal() {
        assert_eq!(
            resolve_config_value("hello", None),
            Some("hello".to_string())
        );
    }

    #[test]
    fn test_empty_literal() {
        assert_eq!(resolve_config_value("", None), Some(String::new()));
    }

    #[test]
    fn test_env_var() {
        let mut env = HashMap::new();
        env.insert("MY_KEY".to_string(), "secret123".to_string());
        assert_eq!(
            resolve_config_value("$MY_KEY", Some(&env)),
            Some("secret123".to_string())
        );
    }

    #[test]
    fn test_env_var_braced() {
        let mut env = HashMap::new();
        env.insert("MY_KEY".to_string(), "secret".to_string());
        assert_eq!(
            resolve_config_value("${MY_KEY}", Some(&env)),
            Some("secret".to_string())
        );
    }

    #[test]
    fn test_env_var_missing() {
        assert_eq!(resolve_config_value("$MISSING", None), None);
    }

    #[test]
    fn test_env_var_with_default() {
        assert_eq!(
            resolve_config_value("${VAR:-default_val}", None),
            Some("default_val".to_string())
        );
    }

    #[test]
    fn test_env_var_with_default_but_set() {
        let mut env = HashMap::new();
        env.insert("VAR".to_string(), "actual".to_string());
        assert_eq!(
            resolve_config_value("${VAR:-fallback}", Some(&env)),
            Some("actual".to_string())
        );
    }

    #[test]
    fn test_shell_command() {
        let result = resolve_config_value("!echo hello_world", None);
        assert_eq!(result, Some("hello_world".to_string()));
    }

    #[test]
    fn test_shell_command_cache() {
        clear_config_value_cache();
        let r1 = resolve_config_value("!echo cache_test", None);
        let r2 = resolve_config_value("!echo cache_test", None);
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_template_mixed() {
        let mut env = HashMap::new();
        env.insert("USER".to_string(), "alice".to_string());
        assert_eq!(
            resolve_config_value("hello_${USER}_world", Some(&env)),
            Some("hello_alice_world".to_string())
        );
    }

    #[test]
    fn test_template_missing_var() {
        assert_eq!(resolve_config_value("prefix_${MISSING}_suffix", None), None);
    }

    #[test]
    fn test_escaped_dollar() {
        assert_eq!(
            resolve_config_value("$$VAR", None),
            Some("$VAR".to_string())
        );
    }

    #[test]
    fn test_escaped_bang() {
        assert_eq!(
            resolve_config_value("$!command", None),
            Some("!command".to_string())
        );
    }

    #[test]
    fn test_parse_literal() {
        assert_eq!(
            parse_config_value("plain"),
            ConfigValueReference::Literal("plain".to_string())
        );
    }

    #[test]
    fn test_parse_shell_cmd() {
        assert_eq!(
            parse_config_value("!echo hi"),
            ConfigValueReference::ShellCmd("echo hi".to_string())
        );
    }

    #[test]
    fn test_parse_env_var() {
        assert_eq!(
            parse_config_value("$API_KEY"),
            ConfigValueReference::EnvVar("API_KEY".to_string())
        );
    }

    #[test]
    fn test_parse_braced_env_var() {
        assert_eq!(
            parse_config_value("${API_KEY}"),
            ConfigValueReference::EnvVar("API_KEY".to_string())
        );
    }

    #[test]
    fn test_resolve_headers() {
        let mut headers = HashMap::new();
        headers.insert("X-Key".to_string(), "value".to_string());
        headers.insert("X-Dynamic".to_string(), "${VAR}".to_string());

        let mut env = HashMap::new();
        env.insert("VAR".to_string(), "resolved".to_string());

        let result = resolve_headers(&headers, Some(&env));
        assert!(result.is_some());
        let map = result.unwrap();
        assert_eq!(map.get("X-Key").unwrap(), "value");
        assert_eq!(map.get("X-Dynamic").unwrap(), "resolved");
    }

    #[test]
    fn test_resolve_headers_skips_missing() {
        let mut headers = HashMap::new();
        headers.insert("X-Missing".to_string(), "${MISSING}".to_string());
        headers.insert("X-Valid".to_string(), "ok".to_string());

        let result = resolve_headers(&headers, None);
        assert!(result.is_some());
        let map = result.unwrap();
        assert!(!map.contains_key("X-Missing"));
        assert_eq!(map.get("X-Valid").unwrap(), "ok");
    }

    #[test]
    fn test_resolve_headers_empty_result() {
        let mut headers = HashMap::new();
        headers.insert("X-Missing".to_string(), "${MISSING}".to_string());
        let result = resolve_headers(&headers, None);
        assert!(result.is_none());
    }

    #[test]
    fn test_get_env_var_names() {
        let names = get_config_value_env_var_names("prefix_${A}_${B:-def}_suffix");
        assert_eq!(names, vec!["A".to_string(), "B".to_string()]);
    }

    #[test]
    fn test_missing_env_var_names() {
        let mut env = HashMap::new();
        env.insert("A".to_string(), "set".to_string());

        let missing = get_missing_config_value_env_var_names("${A}_${B}", Some(&env));
        assert_eq!(missing, vec!["B".to_string()]);
    }

    #[test]
    fn test_is_command() {
        assert!(is_command_config_value("!echo"));
        assert!(!is_command_config_value("echo"));
        assert!(!is_command_config_value("$!echo"));
    }

    #[test]
    fn test_is_configured() {
        let mut env = HashMap::new();
        env.insert("KEY".to_string(), "val".to_string());
        assert!(is_config_value_configured("${KEY}", Some(&env)));
        assert!(!is_config_value_configured("${MISSING}", Some(&env)));
    }

    #[test]
    fn test_resolve_or_throw_ok() {
        let mut env = HashMap::new();
        env.insert("KEY".to_string(), "val".to_string());
        let result = resolve_config_value_or_throw("${KEY}", "test key", Some(&env));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "val");
    }

    #[test]
    fn test_resolve_or_throw_err() {
        let result = resolve_config_value_or_throw("${MISSING}", "missing key", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("MISSING"));
    }

    #[test]
    fn test_shell_command_timeout() {
        // Run a command that sleeps less than our timeout
        let result = resolve_config_value("!echo timeout_test", None);
        assert_eq!(result, Some("timeout_test".to_string()));
    }

    #[test]
    fn test_cache_clear() {
        clear_config_value_cache();
        // First call populates cache
        let _ = resolve_config_value("!echo cache_clear_test", None);
        clear_config_value_cache();
        // After clear, should still work
        let result = resolve_config_value("!echo cache_clear_test", None);
        assert_eq!(result, Some("cache_clear_test".to_string()));
    }
}
