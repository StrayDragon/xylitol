use std::collections::HashMap;
use std::process::Command;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    UnboundVariable { name: String },
    Timeout { command: String },
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

pub const SHELL_BUDGET: Duration = Duration::from_secs(10);

pub const SHELL_CACHE_MAX_ENTRIES: usize = 256;
static SHELL_CACHE: LazyLock<Mutex<HashMap<String, Result<String, ResolveError>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn resolve_from_env(raw: &str) -> Result<String, ResolveError> {
    resolve_value(raw, &|name| std::env::var(name).ok())
}

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

pub fn resolve_shell(command: &str) -> Result<String, ResolveError> {
    {
        let cache = SHELL_CACHE.lock().unwrap();
        if let Some(hit) = cache.get(command).cloned() {
            return hit;
        }
    }
    let result = run_shell_bounded(command, SHELL_BUDGET);
    let mut cache = SHELL_CACHE.lock().unwrap();
    if cache.len() >= SHELL_CACHE_MAX_ENTRIES {
        cache.clear();
    }
    cache.insert(command.to_string(), result.clone());
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

    let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
    let mut reader = child.stdout.take().expect("piped stdout");
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = std::io::Read::read_to_end(&mut reader, &mut buf);
        let _ = tx.send(buf);
    });

    let deadline = Instant::now() + budget;
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break,
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
    let out = rx.recv().unwrap_or_default();
    let trimmed = String::from_utf8_lossy(&out)
        .trim_end_matches(['\n', '\r'])
        .to_string();
    Ok(trimmed)
}

#[allow(dead_code)]
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

    #[test]
    fn shell_large_output_is_not_mistaken_for_timeout() {
        reset_shell_cache();
        let out = resolve_value("!yes x | head -c 200000", &fake_env).unwrap();
        assert!(
            out.len() >= 65_536,
            "大输出 MUST 完整返回而非误报超时（实际长度 {}）",
            out.len()
        );
        assert!(out.starts_with("x\nx\n"), "内容 MUST 未被截断替换");
    }

    #[test]
    fn shell_cache_bounded_cap() {
        reset_shell_cache();
        {
            let mut cache = SHELL_CACHE.lock().unwrap();
            for i in 0..SHELL_CACHE_MAX_ENTRIES {
                cache.insert(format!("!echo {i}"), Ok(format!("hit{i}")));
            }
        }
        assert_eq!(resolve_value("!printf fresh", &fake_env).unwrap(), "fresh");
        let a = resolve_value("!printf %s $$", &fake_env).unwrap();
        let b = resolve_value("!printf %s $$", &fake_env).unwrap();
        assert_eq!(a, b, "清空后对已缓存命令 MUST 仍命中缓存");
    }
}
