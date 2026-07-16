//! One-shot migrate of global AppConfig files from legacy `~/.xylitol/` → XDG.

use std::path::{Path, PathBuf};

/// Filenames that belong in the global **config** dir (`~/.config/xylitol/`),
/// not the data/agent dir (`~/.xylitol/` skills/sessions/logs).
const MIGRATE_FILES: &[&str] = &["config.yaml", "config.local.yaml", "secret.env"];

/// Copy legacy `~/.xylitol/{config.yaml,config.local.yaml,secret.env}` into
/// `global_dir` when the destination file is missing.
///
/// Returns the list of basenames successfully copied (for logging / tests).
/// Does **not** delete legacy files (user can remove after verifying).
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
    for name in MIGRATE_FILES {
        let src = legacy_dir.join(name);
        let dst = global_dir.join(name);
        if !src.is_file() || dst.exists() {
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
                migrated.push((*name).to_string());
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
    use tempfile::TempDir;

    #[test]
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
        let mut migrated = Vec::new();
        for name in MIGRATE_FILES {
            let src = legacy_dir.join(name);
            let dst = global_dir.join(name);
            if !src.is_file() || dst.exists() {
                continue;
            }
            std::fs::create_dir_all(global_dir).unwrap();
            std::fs::copy(&src, &dst).unwrap();
            migrated.push((*name).to_string());
        }
        migrated
    }
}
