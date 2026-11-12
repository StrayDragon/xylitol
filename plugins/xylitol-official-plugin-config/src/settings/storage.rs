use std::path::{Path, PathBuf};

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
}

impl SettingsStorage for FileSettingsStorage {
    fn with_lock(&self, scope: SettingsScope, f: &mut dyn FnMut(Option<&str>) -> Option<String>) {
        let path = match scope {
            SettingsScope::Global => &self.global_path,
            SettingsScope::Project => &self.project_path,
        };
        let lock_path = path.with_extension("json.lock");
        let parent_exists = path.parent().is_some_and(|parent| parent.exists());
        if parent_exists {
            let guard = match xylitol_plugin_sdk::file_lock::FileLockGuard::acquire(lock_path) {
                Ok(guard) => guard,
                Err(e) => {
                    log::error!(
                        target: "xylitol::config",
                        "settings lock acquire failed ({e}); refusing unlocked write"
                    );
                    return;
                }
            };
            let current = std::fs::read_to_string(path).ok();
            let next = f(current.as_deref());
            if let Some(content) = next {
                write_settings(path, &content);
            }
            drop(guard);
            return;
        }
        let Some(content) = f(None) else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let guard = match xylitol_plugin_sdk::file_lock::FileLockGuard::acquire(lock_path) {
            Ok(guard) => guard,
            Err(e) => {
                log::error!(
                    target: "xylitol::config",
                    "settings lock acquire failed ({e}); refusing unlocked write"
                );
                return;
            }
        };
        write_settings(path, &content);
        drop(guard);
    }
}

fn write_settings(path: &Path, content: &str) {
    let tmp = path.with_extension("json.tmp");
    if let Err(e) = std::fs::write(&tmp, format!("{content}\n")) {
        log::warn!("write settings temp file: {e}");
    } else if let Err(e) = std::fs::rename(&tmp, path) {
        log::warn!("rename settings temp file: {e}");
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

    #[test]
    fn concurrent_writes_do_not_lose_updates() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let storage = std::sync::Arc::new(FileSettingsStorage::new(dir.path(), dir.path()));
        let mut handles = Vec::new();
        for _ in 0..8 {
            let storage = std::sync::Arc::clone(&storage);
            handles.push(std::thread::spawn(move || {
                storage.with_lock(SettingsScope::Global, &mut |current| {
                    Some(format!("{}a", current.unwrap_or("")))
                });
            }));
        }
        for handle in handles {
            handle.join().expect("join");
        }
        let content =
            std::fs::read_to_string(dir.path().join("settings.json")).expect("read settings");
        assert_eq!(
            content.matches('a').count(),
            8,
            "locked read-modify-write MUST NOT lose updates: {content:?}"
        );
    }
}
