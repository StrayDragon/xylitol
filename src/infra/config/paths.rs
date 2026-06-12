//! Config directory discovery.
//!
//! Resolves global config dir (XDG / env override), project config dir
//! (CWD walk for `.xylitol/` or `.agents/`), and exposes `ConfigPaths`.

use std::path::{Path, PathBuf};

/// Resolved config paths consumed by the loader and downstream modules.
#[derive(Clone, Debug)]
pub struct ConfigPaths {
    /// Global config directory (e.g. `~/.config/xylitol/`).
    pub global_dir: PathBuf,
    /// Project `.xylitol/` directory, if found.
    pub project_dir: Option<PathBuf>,
    /// Project `.agents/` directory, if found (community convention, read-only).
    pub agents_dir: Option<PathBuf>,
}

impl ConfigPaths {
    /// Discover all config paths.
    ///
    /// Uses:
    /// - `XYLITOL_CONFIG_DIR` env var to override global config dir.
    /// - `XYLITOL_PROJECT_DIR` env var to pin the project root.
    /// - CWD ancestor walk to find `.xylitol/` or `.agents/` as project markers.
    pub fn discover() -> Self {
        let global_dir = resolve_global_dir();
        let (project_dir, agents_dir) = resolve_project_dirs();
        Self {
            global_dir,
            project_dir,
            agents_dir,
        }
    }
}

/// Resolve the global config directory.
///
/// Priority:
/// 1. `$XYLITOL_CONFIG_DIR` env var
/// 2. `$XDG_CONFIG_HOME/xylitol/`
/// 3. `~/.config/xylitol/`
fn resolve_global_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XYLITOL_CONFIG_DIR")
        && !dir.is_empty()
    {
        return PathBuf::from(dir);
    }

    // Try XDG_CONFIG_HOME, fall back to ~/.config
    let base = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg)
    } else if let Some(home) = dirs::home_dir() {
        home.join(".config")
    } else {
        // Last resort: current dir
        return PathBuf::from(".xylitol");
    };

    base.join("xylitol")
}

/// Resolve project directories (`.xylitol/` and `.agents/`).
///
/// If `XYLITOL_PROJECT_DIR` is set, use that directly.
/// Otherwise walk up from CWD looking for `.xylitol/` or `.agents/`.
fn resolve_project_dirs() -> (Option<PathBuf>, Option<PathBuf>) {
    // Explicit override via env.
    if let Ok(dir) = std::env::var("XYLITOL_PROJECT_DIR")
        && !dir.is_empty()
    {
        let root = PathBuf::from(dir);
        let proj = root.join(".xylitol");
        let agents = root.join(".agents");
        return (
            if proj.is_dir() { Some(proj) } else { None },
            if agents.is_dir() { Some(agents) } else { None },
        );
    }

    // Walk up from CWD.
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(_) => return (None, None),
    };

    let mut current: Option<&Path> = Some(cwd.as_path());

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
    use crate::infra::config::test_support::{ENV_LOCK, save_env};

    #[test]
    fn test_resolve_global_dir_env_override() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _guard = save_env(&["XYLITOL_CONFIG_DIR"]);
        // SAFETY: ENV_LOCK held; EnvGuard will restore on drop.
        unsafe { std::env::set_var("XYLITOL_CONFIG_DIR", "/custom/xylitol") };
        let dir = resolve_global_dir();
        assert_eq!(dir, PathBuf::from("/custom/xylitol"));
    }

    #[test]
    fn test_resolve_global_dir_xdg() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _guard = save_env(&["XYLITOL_CONFIG_DIR", "XDG_CONFIG_HOME"]);
        // SAFETY: ENV_LOCK held; EnvGuard will restore on drop.
        unsafe { std::env::remove_var("XYLITOL_CONFIG_DIR") };
        // SAFETY: ENV_LOCK held; EnvGuard will restore on drop.
        unsafe { std::env::set_var("XDG_CONFIG_HOME", "/tmp/xdg-test") };
        let dir = resolve_global_dir();
        assert_eq!(dir, PathBuf::from("/tmp/xdg-test/xylitol"));
    }

    #[test]
    fn test_resolve_global_dir_fallback() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _guard = save_env(&["XYLITOL_CONFIG_DIR", "XDG_CONFIG_HOME"]);
        // SAFETY: ENV_LOCK held; EnvGuard will restore on drop.
        unsafe { std::env::remove_var("XYLITOL_CONFIG_DIR") };
        // SAFETY: ENV_LOCK held; EnvGuard will restore on drop.
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
        // This test verifies it doesn't panic.
        let _dir = resolve_global_dir();
    }
}
