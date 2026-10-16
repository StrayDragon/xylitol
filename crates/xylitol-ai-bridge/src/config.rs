//! Global config dir resolution, shared by the live-provider test binary and
//! the maintenance labs (same priority as the main crate's
//! `infra::config::paths::resolve_global_dir`):
//!
//! 1. `$XYLITOL_CONFIG_DIR`
//! 2. `$XDG_CONFIG_HOME/xylitol`
//! 3. `~/.config/xylitol`

use std::path::PathBuf;

/// Resolve the global config dir from the process environment.
pub fn global_config_dir() -> Option<PathBuf> {
    resolve_global_config_dir(|k| std::env::var(k).ok())
}

/// Injectable variant so tests can drive resolution deterministically without
/// touching the process environment.
pub fn resolve_global_config_dir(get_env: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    if let Some(dir) = get_env("XYLITOL_CONFIG_DIR").filter(|s| !s.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    if let Some(xdg) = get_env("XDG_CONFIG_HOME").filter(|s| !s.is_empty()) {
        return Some(PathBuf::from(xdg).join("xylitol"));
    }
    get_env("HOME")
        .filter(|s| !s.is_empty())
        .map(|home| PathBuf::from(home).join(".config").join("xylitol"))
}

/// Dedicated live-provider config path: explicit env path wins, otherwise the
/// shared `dev/live-provider.yaml` under the global config dir. `None` when
/// neither is available (caller then skips).
pub fn live_provider_config_path(
    explicit: Option<PathBuf>,
    global_dir: Option<PathBuf>,
) -> Option<PathBuf> {
    explicit.or_else(|| global_dir.map(|d| d.join("dev").join("live-provider.yaml")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_map<'a>(entries: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k: &str| {
            entries
                .iter()
                .find(|(name, _)| *name == k)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn global_dir_prefers_xy_config_dir() {
        let env = env_map(&[
            ("XYLITOL_CONFIG_DIR", "/custom/cfg"),
            ("XDG_CONFIG_HOME", "/xdg"),
            ("HOME", "/home/u"),
        ]);
        assert_eq!(
            resolve_global_config_dir(env),
            Some(PathBuf::from("/custom/cfg"))
        );
    }

    #[test]
    fn global_dir_falls_back_to_xdg() {
        let env = env_map(&[("XDG_CONFIG_HOME", "/xdg"), ("HOME", "/home/u")]);
        assert_eq!(
            resolve_global_config_dir(env),
            Some(PathBuf::from("/xdg/xylitol"))
        );
    }

    #[test]
    fn global_dir_falls_back_to_home_config() {
        let env = env_map(&[("HOME", "/home/u")]);
        assert_eq!(
            resolve_global_config_dir(env),
            Some(PathBuf::from("/home/u/.config/xylitol"))
        );
    }

    #[test]
    fn global_dir_none_without_any_anchor() {
        assert_eq!(resolve_global_config_dir(|_| None), None);
        // Empty values are treated as unset.
        let env = env_map(&[("XYLITOL_CONFIG_DIR", ""), ("XDG_CONFIG_HOME", "")]);
        assert_eq!(resolve_global_config_dir(env), None);
    }

    #[test]
    fn live_provider_path_explicit_wins_over_global_dir() {
        let explicit = Some(PathBuf::from("/tmp/explicit.yaml"));
        let global = Some(PathBuf::from("/g"));
        assert_eq!(
            live_provider_config_path(explicit, global),
            Some(PathBuf::from("/tmp/explicit.yaml"))
        );
        // Explicit path wins even when no global dir can be located.
        assert_eq!(
            live_provider_config_path(Some(PathBuf::from("/tmp/explicit.yaml")), None),
            Some(PathBuf::from("/tmp/explicit.yaml"))
        );
    }

    #[test]
    fn live_provider_path_defaults_to_dev_live_provider() {
        assert_eq!(
            live_provider_config_path(None, Some(PathBuf::from("/g"))),
            Some(PathBuf::from("/g/dev/live-provider.yaml"))
        );
        assert_eq!(live_provider_config_path(None, None), None);
    }
}
