//! Config directory discovery.
//!
//! Resolves global config dir (XDG / env override), project config dir
//! (CWD walk for `.xylitol/` or `.agents/`), and exposes `ConfigPaths`.

use std::path::{Path, PathBuf};

/// Resolved config paths consumed by the loader and downstream modules.
#[derive(Clone, Debug)]
pub(crate) struct ConfigPaths {
    /// Global config directory (e.g. `~/.config/xylitol/`).
    pub(crate) global_dir: PathBuf,
    /// Project `.xylitol/` directory, if found.
    pub(crate) project_dir: Option<PathBuf>,
    /// Project `.agents/` directory, if found (community convention, read-only).
    #[allow(dead_code)]
    pub(crate) agents_dir: Option<PathBuf>,
}

impl ConfigPaths {
    /// Discover all config paths from the process environment and CWD.
    ///
    /// Uses:
    /// - `XYLITOL_CONFIG_DIR` env var to override global config dir.
    /// - `XYLITOL_PROJECT_DIR` env var to pin the project root.
    /// - CWD ancestor walk to find `.xylitol/` or `.agents/` as project markers.
    ///
    /// Global **AppConfig** SSOT is `~/.config/xylitol/` (or XDG / env override),
    /// **not** `~/.xylitol/` (that remains the data/agent dir for skills/sessions/logs).
    /// On discover, missing global config files are one-shot copied from legacy
    /// `~/.xylitol/{config.yaml,config.yml,secret.env}` when present (`config.local.*` skipped).
    pub(crate) fn discover() -> Self {
        Self::discover_with(
            |k| {
                if let Ok(v) = std::env::var(k) {
                    return Some(v);
                }
                // Match resolve_global_dir / legacy migrate: dirs when HOME unset.
                if k == "HOME" {
                    return dirs::home_dir().map(|p| p.to_string_lossy().into_owned());
                }
                None
            },
            std::env::current_dir().ok().as_deref(),
        )
    }

    /// Injectable discovery for tests (no `std::env::set_var`).
    ///
    /// `get_env` should return `None` for unset keys. `cwd` is used only when
    /// `XYLITOL_PROJECT_DIR` is unset.
    pub(crate) fn discover_with(
        get_env: impl Fn(&str) -> Option<String>,
        cwd: Option<&Path>,
    ) -> Self {
        let global_dir = resolve_global_dir_with(&get_env);
        super::migrate::migrate_legacy_global_config_files_with(&global_dir, &get_env);
        let (project_dir, agents_dir) = resolve_project_dirs_with(&get_env, cwd);
        Self {
            global_dir,
            project_dir,
            agents_dir,
        }
    }
}

/// Resolve the global config directory from an env getter.
///
/// Priority:
/// 1. `$XYLITOL_CONFIG_DIR`
/// 2. `$XDG_CONFIG_HOME/xylitol/`
/// 3. `$HOME/.config/xylitol/` (or `dirs::home_dir` when `HOME` unset)
fn resolve_global_dir_with(get_env: &impl Fn(&str) -> Option<String>) -> PathBuf {
    if let Some(dir) = get_env("XYLITOL_CONFIG_DIR").filter(|s| !s.is_empty()) {
        return PathBuf::from(dir);
    }

    let base = if let Some(xdg) = get_env("XDG_CONFIG_HOME").filter(|s| !s.is_empty()) {
        PathBuf::from(xdg)
    } else if let Some(home) = get_env("HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
    {
        home.join(".config")
    } else {
        return PathBuf::from(".xylitol");
    };

    base.join("xylitol")
}

fn resolve_project_dirs_with(
    get_env: &impl Fn(&str) -> Option<String>,
    cwd: Option<&Path>,
) -> (Option<PathBuf>, Option<PathBuf>) {
    if let Some(dir) = get_env("XYLITOL_PROJECT_DIR").filter(|s| !s.is_empty()) {
        let root = PathBuf::from(dir);
        let proj = root.join(".xylitol");
        let agents = root.join(".agents");
        return (
            if proj.is_dir() { Some(proj) } else { None },
            if agents.is_dir() { Some(agents) } else { None },
        );
    }

    let cwd = match cwd {
        Some(d) => d,
        None => return (None, None),
    };

    let mut current: Option<&Path> = Some(cwd);

    while let Some(dir) = current {
        let proj = dir.join(".xylitol");
        let agents = dir.join(".agents");

        let has_proj = proj.is_dir();
        let has_agents = agents.is_dir();

        if has_proj || has_agents {
            return (
                if has_proj { Some(proj) } else { None },
                if has_agents { Some(agents) } else { None },
            );
        }

        current = dir.parent();
    }

    (None, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_map<'a>(entries: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k: &str| {
            entries
                .iter()
                .find(|(name, _)| *name == k)
                .map(|(_, v)| (*v).to_string())
        }
    }

    #[test]
    fn discover_with_prefers_xy_config_dir() {
        let home = tempfile::tempdir().unwrap();
        let custom = home.path().join("custom-cfg");
        std::fs::create_dir_all(&custom).unwrap();
        let custom_s = custom.to_str().unwrap().to_string();
        let entries = [
            ("XYLITOL_CONFIG_DIR", custom_s.as_str()),
            ("XDG_CONFIG_HOME", "/xdg"),
            ("HOME", "/home/u"),
        ];
        let env = env_map(&entries);
        let paths = ConfigPaths::discover_with(env, None);
        assert_eq!(paths.global_dir, custom);
        assert!(paths.project_dir.is_none());
    }

    #[test]
    fn discover_with_project_dir_env() {
        let root = tempfile::tempdir().unwrap();
        let proj = root.path().join(".xylitol");
        std::fs::create_dir_all(&proj).unwrap();
        let global = root.path().join("global");
        std::fs::create_dir_all(&global).unwrap();
        let global_s = global.to_str().unwrap().to_string();
        let root_s = root.path().to_str().unwrap().to_string();
        let entries = [
            ("XYLITOL_CONFIG_DIR", global_s.as_str()),
            ("XYLITOL_PROJECT_DIR", root_s.as_str()),
        ];
        let env = env_map(&entries);
        let paths = ConfigPaths::discover_with(env, None);
        assert_eq!(paths.project_dir.as_deref(), Some(proj.as_path()));
    }

    #[test]
    fn discover_with_walks_cwd_when_no_project_env() {
        let root = tempfile::tempdir().unwrap();
        let nested = root.path().join("a").join("b");
        std::fs::create_dir_all(&nested).unwrap();
        let proj = root.path().join(".xylitol");
        std::fs::create_dir_all(&proj).unwrap();
        let global = root.path().join("global");
        std::fs::create_dir_all(&global).unwrap();
        let global_s = global.to_str().unwrap().to_string();
        let entries = [("XYLITOL_CONFIG_DIR", global_s.as_str())];
        let env = env_map(&entries);
        let paths = ConfigPaths::discover_with(env, Some(nested.as_path()));
        assert_eq!(paths.project_dir.as_deref(), Some(proj.as_path()));
    }
}
