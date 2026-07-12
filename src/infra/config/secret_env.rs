//! Collect + inject `secret.env` dotenv files (runtime-config r5).
//!
//! Global then project; project overlays global. OS/shell env always wins
//! for `std::env` injection. The collected map is also the `secret.*`
//! namespace for YAML template rendering (`{{ secret.KEY }}`).

use std::collections::HashMap;
use std::path::Path;

use super::paths::ConfigPaths;

/// Secrets loaded from dotenv files (global ← project overlay).
pub(crate) type SecretMap = HashMap<String, String>;

/// Read `secret.env` maps from config locations (does not touch process env).
pub(crate) fn collect_secret_map(paths: &ConfigPaths) -> SecretMap {
    let mut map = HashMap::new();

    let global = paths.global_dir.join("secret.env");
    if global.is_file() {
        merge_dotenv_file(&global, &mut map);
    }

    if let Some(ref proj) = paths.project_dir {
        let local = proj.join("secret.env");
        if local.is_file() {
            merge_dotenv_file(&local, &mut map);
        }
    }

    map
}

/// Collect secrets and inject missing keys into the process environment.
///
/// Returns `(secret_map, newly_injected_count)`.
pub(crate) fn load_secret_env_files(paths: &ConfigPaths) -> (SecretMap, usize) {
    let map = collect_secret_map(paths);
    let injected = inject_missing_env(&map);
    (map, injected)
}

fn merge_dotenv_file(path: &Path, map: &mut HashMap<String, String>) {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return;
    };
    for (key, value) in parse_dotenv(&raw) {
        map.insert(key, value);
    }
}

fn inject_missing_env(map: &HashMap<String, String>) -> usize {
    let mut set = 0;
    for (key, value) in map {
        if std::env::var_os(key).is_some() {
            continue;
        }
        // SAFETY: single-threaded at CLI startup before worker threads.
        unsafe {
            std::env::set_var(key, value);
        }
        set += 1;
    }
    set
}

/// Minimal dotenv parser: `#` comments, blank lines, `KEY=VALUE`, optional quotes.
pub(crate) fn parse_dotenv(raw: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() || key.contains(char::is_whitespace) {
            continue;
        }
        let value = strip_dotenv_quotes(value.trim());
        out.push((key.to_string(), value));
    }
    out
}

fn strip_dotenv_quotes(value: &str) -> String {
    let bytes = value.as_bytes();
    if bytes.len() >= 2 {
        let (a, b) = (bytes[0], bytes[bytes.len() - 1]);
        if (a == b'"' && b == b'"') || (a == b'\'' && b == b'\'') {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn parse_skips_comments_and_blanks() {
        let pairs = parse_dotenv(
            "# comment\n\nOPENAI_API_KEY=sk-abc\nANTHROPIC_API_KEY=\"sk-xyz\"\nBAD LINE\n",
        );
        assert_eq!(
            pairs,
            vec![
                ("OPENAI_API_KEY".into(), "sk-abc".into()),
                ("ANTHROPIC_API_KEY".into(), "sk-xyz".into()),
            ]
        );
    }

    #[test]
    fn inject_does_not_override_existing_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        let key = "XYLITOL_TEST_SECRET_ENV_NO_OVERRIDE";
        unsafe {
            std::env::set_var(key, "from-shell");
        }
        let mut map = HashMap::new();
        map.insert(key.to_string(), "from-file".into());
        assert_eq!(inject_missing_env(&map), 0);
        assert_eq!(std::env::var(key).unwrap(), "from-shell");
        unsafe {
            std::env::remove_var(key);
        }
    }

    #[test]
    fn inject_sets_missing_keys() {
        let _guard = ENV_LOCK.lock().unwrap();
        let key = "XYLITOL_TEST_SECRET_ENV_SET";
        unsafe {
            std::env::remove_var(key);
        }
        let mut map = HashMap::new();
        map.insert(key.to_string(), "from-file".into());
        assert_eq!(inject_missing_env(&map), 1);
        assert_eq!(std::env::var(key).unwrap(), "from-file");
        unsafe {
            std::env::remove_var(key);
        }
    }

    #[test]
    fn load_from_temp_project_dir() {
        let _guard = ENV_LOCK.lock().unwrap();
        let key = "XYLITOL_TEST_SECRET_ENV_LOAD";
        unsafe {
            std::env::remove_var(key);
        }

        let tmp = tempfile::tempdir().unwrap();
        let proj = tmp.path().join(".xylitol");
        fs::create_dir_all(&proj).unwrap();
        fs::write(proj.join("secret.env"), format!("{key}=loaded-ok\n")).unwrap();

        let paths = ConfigPaths {
            global_dir: tmp.path().join("no-global"),
            project_dir: Some(proj),
            agents_dir: None,
        };
        let (map, injected) = load_secret_env_files(&paths);
        assert_eq!(injected, 1);
        assert_eq!(map.get(key).map(String::as_str), Some("loaded-ok"));
        assert_eq!(std::env::var(key).unwrap(), "loaded-ok");
        unsafe {
            std::env::remove_var(key);
        }
    }
}
