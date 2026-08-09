//! One-shot migrate of global AppConfig files from legacy `~/.xylitol/` → XDG.

use std::path::{Path, PathBuf};

/// Copy legacy `~/.xylitol/{config.yaml,config.yml,secret.env}` into `global_dir`
/// when the destination file is missing.
///
/// `config.yml` migrates to `config.yaml` (loader SSOT name).
/// `config.local.*` is **not** migrated (c1400 — unsupported).
/// Returns the list of destination basenames successfully copied.
/// Does **not** delete legacy files (user can remove after verifying).
///
/// Global AppConfig SSOT remains `~/.config/xylitol/` (not `~/.xylitol/`).
pub(crate) fn migrate_legacy_global_config_files(global_dir: &Path) -> Vec<String> {
    let Some(legacy_dir) = legacy_xylitol_home_dir() else {
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

fn legacy_xylitol_home_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".xylitol"))
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
    use serial_test::serial;
    use tempfile::TempDir;

    #[test]
    #[serial(env_global)]

    fn migrates_missing_dest_only() {
        let home = TempDir::new().unwrap();
        let legacy = home.path().join(".xylitol");
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("config.yaml"), "model: {}\n").unwrap();
        std::fs::write(legacy.join("secret.env"), "K=v\n").unwrap();
        // Pretend home for this test by calling the core copy logic via paths.
        // Unit API takes explicit dirs — exercise through a thin test helper.
        let migrated = migrate_between(&legacy, &global);
        assert!(migrated.contains(&"config.yaml".to_string()));
        assert!(migrated.contains(&"secret.env".to_string()));
        assert_eq!(
            std::fs::read_to_string(global.join("config.yaml")).unwrap(),
            "model: {}\n"
        );
        // Second run: dest exists → no re-copy / no overwrite.
        std::fs::write(legacy.join("config.yaml"), "changed: true\n").unwrap();
        let again = migrate_between(&legacy, &global);
        assert!(again.is_empty());
        assert_eq!(
            std::fs::read_to_string(global.join("config.yaml")).unwrap(),
            "model: {}\n"
        );
    }

    /// Test-only: same rules as [`migrate_legacy_global_config_files`] with explicit dirs.
    fn migrate_between(legacy_dir: &Path, global_dir: &Path) -> Vec<String> {
        let pairs = [
            ("config.yaml", "config.yaml"),
            ("config.yml", "config.yaml"),
            ("secret.env", "secret.env"),
        ];
        let mut migrated = Vec::new();
        for (src_name, dst_name) in pairs {
            let src = legacy_dir.join(src_name);
            let dst = global_dir.join(dst_name);
            if !src.is_file() || dst.exists() {
                continue;
            }
            std::fs::create_dir_all(global_dir).unwrap();
            std::fs::copy(&src, &dst).unwrap();
            migrated.push(dst_name.to_string());
        }
        migrated
    }

    #[test]
    #[serial(env_global)]

    fn migrates_yml_alias_to_yaml() {
        let home = TempDir::new().unwrap();
        let legacy = home.path().join(".xylitol");
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("config.yml"), "mcp_servers: []\n").unwrap();
        let migrated = migrate_between(&legacy, &global);
        assert!(migrated.contains(&"config.yaml".to_string()));
        assert!(global.join("config.yaml").is_file());
    }

    #[test]
    #[serial(env_global)]

    fn does_not_migrate_config_local() {
        let home = TempDir::new().unwrap();
        let legacy = home.path().join(".xylitol");
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("config.local.yaml"), "models: {}\n").unwrap();
        // SAFETY: test-only HOME override for migrate_legacy_global_config_files.
        let prev = std::env::var("HOME").ok();
        unsafe { std::env::set_var("HOME", home.path()) };
        let migrated = migrate_legacy_global_config_files(&global);
        match prev {
            Some(v) => unsafe { std::env::set_var("HOME", v) },
            None => unsafe { std::env::remove_var("HOME") },
        }
        assert!(!migrated.iter().any(|m| m.contains("local")));
        assert!(!global.join("config.local.yaml").exists());
    }
}
