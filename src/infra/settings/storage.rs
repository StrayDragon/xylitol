//! File-backed settings storage with retry-locking.
//!
//! Supports global `~/.xylitol/settings.json` and project `<cwd>/.xylitol/settings.json`
//! with proper-lockfile semantics.
//! Rust doesn't have proper-lockfile, so we use atomic temp-file + rename.

use std::path::{Path, PathBuf};

/// Settings lock acquisition failures (crate-private; not `Xy*`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
struct SettingsLockError(String);

impl From<&str> for SettingsLockError {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for SettingsLockError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

pub trait SettingsStorage {
    fn with_lock(&self, scope: SettingsScope, f: &mut dyn FnMut(Option<&str>) -> Option<String>);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsScope {
    Global,
    Project,
}

/// File-based settings storage with retry locking.
#[derive(Debug, Clone)]
pub struct FileSettingsStorage {
    global_path: PathBuf,
    project_path: PathBuf,
}

impl FileSettingsStorage {
    pub fn new(cwd: &Path, agent_dir: &Path) -> Self {
        Self {
            global_path: agent_dir.join("settings.json"),
            project_path: cwd.join(".xylitol").join("settings.json"),
        }
    }

    fn acquire_with_retry(&self, path: &Path) -> Result<(), SettingsLockError> {
        let max_attempts = 10;
        let delay_us = 20_000; // 20ms

        for attempt in 1..=max_attempts {
            // Try to create a lock file atomically
            let lock_path = path.with_extension("json.lock");
            match std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&lock_path)
            {
                Ok(_lock) => {
                    // We hold the lock
                    return Ok(());
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    if attempt == max_attempts {
                        return Err(format!(
                            "Failed to acquire settings lock after {max_attempts} attempts"
                        )
                        .into());
                    }
                    // Wait and retry
                    std::thread::sleep(std::time::Duration::from_micros(delay_us));
                }
                Err(e) => {
                    return Err(format!("Lock error: {e}").into());
                }
            }
        }
        Err("unreachable".into())
    }

    fn release(&self, path: &Path) {
        let lock_path = path.with_extension("json.lock");
        let _ = std::fs::remove_file(lock_path);
    }
}

impl SettingsStorage for FileSettingsStorage {
    fn with_lock(&self, scope: SettingsScope, f: &mut dyn FnMut(Option<&str>) -> Option<String>) {
        let path = match scope {
            SettingsScope::Global => &self.global_path,
            SettingsScope::Project => &self.project_path,
        };

        // Ensure parent dir exists
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Try to acquire lock (best-effort)
        let locked = self.acquire_with_retry(path).is_ok();

        let current = std::fs::read_to_string(path).ok();
        let current_ref = current.as_deref();
        let next = f(current_ref);

        if let Some(content) = next {
            // Atomic write: temp file + rename
            let tmp = path.with_extension("json.tmp");
            if let Err(e) = std::fs::write(&tmp, format!("{content}\n")) {
                log::warn!("write settings temp file: {e}");
            } else if let Err(e) = std::fs::rename(&tmp, path) {
                log::warn!("rename settings temp file: {e}");
            }
        }

        if locked {
            self.release(path);
        }
    }
}

/// In-memory storage for testing.
#[derive(Debug)]
pub struct InMemorySettingsStorage {
    pub global: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    pub project: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

impl Clone for InMemorySettingsStorage {
    fn clone(&self) -> Self {
        Self {
            global: std::sync::Arc::clone(&self.global),
            project: std::sync::Arc::clone(&self.project),
        }
    }
}

impl Default for InMemorySettingsStorage {
    fn default() -> Self {
        Self {
            global: std::sync::Arc::new(std::sync::Mutex::new(None)),
            project: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

impl SettingsStorage for InMemorySettingsStorage {
    fn with_lock(&self, scope: SettingsScope, f: &mut dyn FnMut(Option<&str>) -> Option<String>) {
        let cell = match scope {
            SettingsScope::Global => &self.global,
            SettingsScope::Project => &self.project,
        };
        let mut guard = cell.lock().unwrap();
        let current = guard.clone();
        let ref_opt = current.as_deref();
        let next = f(ref_opt);
        if let Some(content) = next {
            *guard = Some(content);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_save_and_load() {
        let storage = InMemorySettingsStorage::default();
        storage.with_lock(SettingsScope::Global, &mut |_current| {
            Some(r#"{"defaultModel":"claude-3"}"#.into())
        });
        storage.with_lock(SettingsScope::Global, &mut |current| {
            assert_eq!(current, Some(r#"{"defaultModel":"claude-3"}"#));
            None
        });
    }

    #[test]
    fn test_file_storage_roundtrip() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let storage = FileSettingsStorage::new(dir.path(), dir.path());

        storage.with_lock(SettingsScope::Global, &mut |_current| {
            Some(r#"{"defaultModel":"gpt-4o"}"#.into())
        });

        let mut found = String::new();
        storage.with_lock(SettingsScope::Global, &mut |current| {
            found = current.unwrap_or_default().to_string();
            None
        });
        assert!(found.contains("gpt-4o"));
    }
}
