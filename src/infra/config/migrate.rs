//! One-shot migrate of global AppConfig files from legacy `~/.xylitol/` → XDG.

use std::path::{Path, PathBuf};

/// Injectable migrate: legacy dir is `$HOME/.xylitol` from `get_env` (empty/`None` → no-op).
///
/// `config.yml` migrates to `config.yaml` (loader SSOT name).
/// `config.local.*` is **not** migrated (c1400 — unsupported).
/// Returns the list of destination basenames successfully copied.
/// Does **not** delete legacy files (user can remove after verifying).
///
/// Global AppConfig SSOT remains `~/.config/xylitol/` (not `~/.xylitol/`).
/// Process discovery supplies HOME via env or `dirs` (see [`super::paths::ConfigPaths::discover`]).
pub(crate) fn migrate_legacy_global_config_files_with(
    global_dir: &Path,
    get_env: impl Fn(&str) -> Option<String>,
) -> Vec<String> {
    let Some(legacy_dir) = legacy_xylitol_home_dir_with(&get_env) else {
        return Vec::new();
    };
    if !legacy_dir.is_dir() {
        return Vec::new();
    }
    // Same path (tests / XYLITOL_CONFIG_DIR pointing at ~/.xylitol) → no-op.
    if paths_equal(&legacy_dir, global_dir) {
        return Vec::new();
    }

    let mut migrated = Vec::new();
    // Prefer config.yaml; fall back to legacy config.yml → config.yaml.
    let yaml_pairs = [
        ("config.yaml", "config.yaml"),
        ("config.yml", "config.yaml"),
        ("secret.env", "secret.env"),
    ];
    for (src_name, dst_name) in yaml_pairs {
        let src = legacy_dir.join(src_name);
        let dst = global_dir.join(dst_name);
        if !src.is_file() || dst.exists() {
            continue;
        }
        // Skip config.yml if config.yaml already migrated in this pass.
        if src_name == "config.yml" && migrated.iter().any(|m| m == "config.yaml") {
            continue;
        }
        if let Err(e) = std::fs::create_dir_all(global_dir) {
            log::warn!(
                target: "xylitol::config",
                "migrate: cannot create global config dir {}: {e}",
                global_dir.display()
            );
            return migrated;
        }
        match std::fs::copy(&src, &dst) {
            Ok(_) => {
                log::info!(
                    target: "xylitol::config",
                    "migrated legacy global config {} → {}",
                    src.display(),
                    dst.display()
                );
                migrated.push(dst_name.to_string());
            }
            Err(e) => {
                log::warn!(
                    target: "xylitol::config",
                    "migrate: failed to copy {} → {}: {e}",
                    src.display(),
                    dst.display()
                );
            }
        }
    }
    migrated
}

fn legacy_xylitol_home_dir_with(get_env: &impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    // Injectable path: only honor get_env (None/empty → no migrate).
    get_env("HOME")
        .filter(|s| !s.is_empty())
        .map(|h| PathBuf::from(h).join(".xylitol"))
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn migrates_missing_dest_only() {
        let home = TempDir::new().unwrap();
        let legacy = home.path().join(".xylitol");
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("config.yaml"), "model: {}\n").unwrap();
        std::fs::write(legacy.join("secret.env"), "K=v\n").unwrap();
        let home_s = home.path().to_str().unwrap().to_string();
        let migrated = migrate_legacy_global_config_files_with(&global, |k| {
            (k == "HOME").then(|| home_s.clone())
        });
        assert!(migrated.contains(&"config.yaml".to_string()));
        assert!(migrated.contains(&"secret.env".to_string()));
        assert_eq!(
            std::fs::read_to_string(global.join("config.yaml")).unwrap(),
            "model: {}\n"
        );
        // Second run: dest exists → no re-copy / no overwrite.
        std::fs::write(legacy.join("config.yaml"), "changed: true\n").unwrap();
        let again = migrate_legacy_global_config_files_with(&global, |k| {
            (k == "HOME").then(|| home_s.clone())
        });
        assert!(again.is_empty());
        assert_eq!(
            std::fs::read_to_string(global.join("config.yaml")).unwrap(),
            "model: {}\n"
        );
    }

    #[test]
    fn migrates_yml_alias_to_yaml() {
        let home = TempDir::new().unwrap();
        let legacy = home.path().join(".xylitol");
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("config.yml"), "mcp_servers: []\n").unwrap();
        let home_s = home.path().to_str().unwrap().to_string();
        let migrated = migrate_legacy_global_config_files_with(&global, |k| {
            (k == "HOME").then(|| home_s.clone())
        });
        assert!(migrated.contains(&"config.yaml".to_string()));
        assert!(global.join("config.yaml").is_file());
    }

    #[test]
    fn does_not_migrate_config_local() {
        let home = TempDir::new().unwrap();
        let legacy = home.path().join(".xylitol");
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("config.local.yaml"), "models: {}\n").unwrap();
        let home_s = home.path().to_str().unwrap().to_string();
        let migrated = migrate_legacy_global_config_files_with(&global, |k| {
            (k == "HOME").then(|| home_s.clone())
        });
        assert!(!migrated.iter().any(|m| m.contains("local")));
        assert!(!global.join("config.local.yaml").exists());
    }
}
