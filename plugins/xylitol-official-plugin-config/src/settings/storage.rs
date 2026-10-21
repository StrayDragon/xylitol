use std::path::{Path, PathBuf};

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
        let delay_us = 20_000;

        for attempt in 1..=max_attempts {
            let lock_path = path.with_extension("json.lock");
            match std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&lock_path)
            {
                Ok(_lock) => {
                    return Ok(());
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    if attempt == max_attempts {
                        return Err(format!(
                            "Failed to acquire settings lock after {max_attempts} attempts"
                        )
                        .into());
                    }
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

        let current = std::fs::read_to_string(path).ok();
        let current_ref = current.as_deref();
        let next = f(current_ref);

        if let Some(content) = next {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let locked = self.acquire_with_retry(path).is_ok();
            let tmp = path.with_extension("json.tmp");
            if let Err(e) = std::fs::write(&tmp, format!("{content}\n")) {
                log::warn!("write settings temp file: {e}");
            } else if let Err(e) = std::fs::rename(&tmp, path) {
                log::warn!("rename settings temp file: {e}");
            }
            if locked {
                self.release(path);
            }
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
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

    #[test]
    fn read_does_not_create_project_xylitol_dir() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let project = dir.path().join("proj");
        std::fs::create_dir_all(&project).unwrap();
        let agent = dir.path().join("agent");
        std::fs::create_dir_all(&agent).unwrap();
        let storage = FileSettingsStorage::new(&project, &agent);
        storage.with_lock(SettingsScope::Project, &mut |_current| None);
        assert!(
            !project.join(".xylitol").exists(),
            "reading project settings must not mkdir <cwd>/.xylitol"
        );
    }
}
