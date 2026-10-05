//! Config value resolver — literal / env-var template / shell-command values
//! (r1821-1823).
//!
//! Grammar (per spec):
//! - literal:                no marker → as-is
//! - env-var template:       `$VAR`, `${VAR}`, `${VAR:-default}`
//! - shell command:          prefix `!` → run under `sh -c` with a 10s wall-clock
//!   budget; successful results are cached for the process lifetime; an expired
//!   budget yields a `Timeout` (already-classified failure), never an infinite
//!   wait.
//!
//! Pure/std-only — no crate-internal dependencies.

use std::collections::HashMap;
use std::process::Command;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

/// Classified value-resolution failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// `$VAR` / `${VAR}` with no `:-default` and no env binding.
    UnboundVariable { name: String },
    /// `!command` exceeded the wall-clock budget.
    Timeout { command: String },
    /// `!command` could not be spawned or joined.
    Spawn { command: String },
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnboundVariable { name } => write!(f, "unbound config value: {name}"),
            Self::Timeout { command } => write!(f, "config value command timed out: {command}"),
            Self::Spawn { command } => write!(f, "config value command failed: {command}"),
        }
    }
}

impl std::error::Error for ResolveError {}

/// Wall clock budget for `!command` values (spec r1823).
pub const SHELL_BUDGET: Duration = Duration::from_secs(10);

/// Successful shell results are cached for the process lifetime.
static SHELL_CACHE: LazyLock<Mutex<HashMap<String, Result<String, ResolveError>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Classify a raw config value string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigValue {
    Literal(String),
    Env {
        name: String,
        default: Option<String>,
    },
    Shell {
        command: String,
    },
}

/// Classify without resolving (pure parse).
pub fn parse_value(raw: &str) -> ConfigValue {
    if let Some(cmd) = raw.strip_prefix('!') {
        return ConfigValue::Shell {
            command: cmd.to_string(),
        };
    }
    if let Some(inner) = raw.strip_prefix("${") {
        let Some(close) = inner.find('}') else {
            return ConfigValue::Literal(raw.to_string());
        };
        let body = &inner[..close];
        return match body.find(":-") {
            Some(sep) => ConfigValue::Env {
                name: body[..sep].to_string(),
                default: Some(body[sep + 2..].to_string()),
            },
            None => ConfigValue::Env {
                name: body.to_string(),
                default: None,
            },
        };
    }
    if let Some(name) = raw.strip_prefix('$') {
        return ConfigValue::Env {
            name: name.to_string(),
            default: None,
        };
    }
    ConfigValue::Literal(raw.to_string())
}

/// Resolve a raw value with the given env lookup.
pub fn resolve_value(
    raw: &str,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<String, ResolveError> {
    match parse_value(raw) {
        ConfigValue::Literal(v) => Ok(v),
        ConfigValue::Env { name, default } => match lookup(&name) {
            Some(v) => Ok(v),
            None => default
                .or_else(|| std::env::var(&name).ok())
                .ok_or_else(|| ResolveError::UnboundVariable { name: name.clone() }),
        },
        ConfigValue::Shell { command } => resolve_shell(&command),
    }
}

/// Resolve a `!command` value: bounded execution + process-lifetime cache.
pub fn resolve_shell(command: &str) -> Result<String, ResolveError> {
    if let Some(hit) = SHELL_CACHE.lock().unwrap().get(command).cloned() {
        return hit;
    }
    let result = run_shell_bounded(command, SHELL_BUDGET);
    SHELL_CACHE
        .lock()
        .unwrap()
        .insert(command.to_string(), result.clone());
    result
}

fn run_shell_bounded(command: &str, budget: Duration) -> Result<String, ResolveError> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|_| ResolveError::Spawn {
            command: command.to_string(),
        })?;

    let deadline = Instant::now() + budget;
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                let mut out = String::new();
                use std::io::Read;
                if let Some(mut stdout) = child.stdout.take() {
                    let _ = stdout.read_to_string(&mut out);
                }
                let trimmed = out.trim_end_matches(['\n', '\r']).to_string();
                return Ok(if trimmed.is_empty() {
                    String::new()
                } else {
                    trimmed
                });
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(ResolveError::Timeout {
                        command: command.to_string(),
                    });
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => {
                return Err(ResolveError::Spawn {
                    command: command.to_string(),
                });
            }
        }
    }
}

/// Clear the shell cache (tests / reload).
pub fn reset_shell_cache() {
    SHELL_CACHE.lock().unwrap().clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_env(name: &str) -> Option<String> {
        match name {
            "HOME" => Some("/home/u".into()),
            _ => None,
        }
    }

    #[test]
    fn literal_passes_through() {
        assert_eq!(
            parse_value("plain string"),
            ConfigValue::Literal("plain string".into())
        );
        assert_eq!(
            resolve_value("plain string", &fake_env).unwrap(),
            "plain string"
        );
    }

    #[test]
    fn dollar_var_resolves() {
        assert_eq!(
            parse_value("$HOME"),
            ConfigValue::Env {
                name: "HOME".into(),
                default: None
            }
        );
        assert_eq!(resolve_value("$HOME", &fake_env).unwrap(), "/home/u");
    }

    #[test]
    fn braced_var_with_default() {
        let v = parse_value("${UNSET:-fallback}");
        assert_eq!(
            v,
            ConfigValue::Env {
                name: "UNSET".into(),
                default: Some("fallback".into())
            }
        );
        assert_eq!(
            resolve_value("${UNSET:-fallback}", &fake_env).unwrap(),
            "fallback"
        );
    }

    #[test]
    fn unbound_without_default_is_error() {
        let err = resolve_value("${NOPE}", &fake_env).unwrap_err();
        assert_eq!(
            err,
            ResolveError::UnboundVariable {
                name: "NOPE".into()
            }
        );
    }

    #[test]
    fn shell_command_executes() {
        reset_shell_cache();
        assert_eq!(
            parse_value("!printf ok"),
            ConfigValue::Shell {
                command: "printf ok".into()
            }
        );
        assert_eq!(resolve_value("!printf ok", &fake_env).unwrap(), "ok");
    }

    #[test]
    fn shell_result_is_cached_per_process() {
        reset_shell_cache();
        // `$$` 是 shell PID：进程内每次执行本应不同；缓存命中则两次相同。
        let a = resolve_value("!printf %s $$", &fake_env).unwrap();
        let b = resolve_value("!printf %s $$", &fake_env).unwrap();
        assert_eq!(a, b, "进程生命周期内结果 MUST 缓存");
    }

    #[test]
    fn shell_timeout_is_classified() {
        reset_shell_cache();
        let started = Instant::now();
        let err = run_shell_bounded("sleep 30", Duration::from_millis(120)).unwrap_err();
        assert!(matches!(err, ResolveError::Timeout { .. }));
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "MUST 按时收口而非挂起"
        );
    }
}
