use std::fs;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

fn valid_segment(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

pub const KEY_EDITOR_HISTORY: &str = "editor-history";

const EDITOR_HISTORY_CAP: usize = 100;

#[derive(Debug, Clone)]
pub struct ClientStorage {
    dir: PathBuf,
}

impl ClientStorage {
    pub fn from_env() -> Self {
        let root = std::env::var("XYLITOL_STATE_ROOT")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".xylitol")
                    .join("state")
            });
        let channel = std::env::var("XYLITOL_CHANNEL")
            .ok()
            .filter(|s| valid_segment(s))
            .unwrap_or_else(|| "default".into());
        Self::new(root.clone(), &channel)
            .unwrap_or_else(|_| Self::new(root, "default").expect("default"))
    }

    pub fn new(root: impl Into<PathBuf>, channel: &str) -> Result<Self, String> {
        if !valid_segment(channel) {
            return Err(format!("invalid channel segment: {channel}"));
        }
        Ok(Self {
            dir: root.into().join(channel).join("tui"),
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn key_path(&self, key: &str) -> Result<PathBuf, String> {
        if !valid_segment(key) {
            return Err(format!("invalid key segment: {key}"));
        }
        Ok(self.dir.join(format!("{key}.json")))
    }

    pub fn load<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let path = self.key_path(key).ok()?;
        let raw = fs::read_to_string(path).ok()?;
        serde_json::from_str(&raw).ok()
    }

    pub fn update<T, F>(&self, key: &str, default: T, mutate: F) -> Result<T, String>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce(&mut T),
    {
        let path = self.key_path(key)?;
        fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        let lock_path = self.dir.join(format!("{key}.lock"));
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|e| e.to_string())?;
        let fd = lock.as_raw_fd();
        let rc = unsafe { libc::flock(fd, libc::LOCK_EX) };
        if rc != 0 {
            return Err("flock failed".into());
        }
        let result = (|| {
            let mut val = self.load(key).unwrap_or(default);
            mutate(&mut val);
            let serialized = serde_json::to_string(&val).map_err(|e| e.to_string())?;
            let tmp = self
                .dir
                .join(format!(".{}.{}.tmp", key, uuid::Uuid::new_v4().simple()));
            let persisted = (|| {
                let mut f = fs::File::create(&tmp).map_err(|e| e.to_string())?;
                f.write_all(serialized.as_bytes())
                    .map_err(|e| e.to_string())?;
                f.sync_all().ok();
                fs::rename(&tmp, &path).map_err(|e| e.to_string())
            })();
            if persisted.is_err() {
                let _ = fs::remove_file(&tmp);
            }
            persisted?;
            Ok(val)
        })();
        unsafe {
            libc::flock(fd, libc::LOCK_UN);
        }
        result
    }

    pub fn record_editor_prompt(&self, text: &str) {
        let text = text.trim().to_string();
        if text.is_empty() {
            return;
        }
        let _ = self.update::<Vec<String>, _>(KEY_EDITOR_HISTORY, Vec::new(), |v| {
            v.retain(|t| t != &text);
            v.insert(0, text.clone());
            v.truncate(EDITOR_HISTORY_CAP);
        });
    }

    pub fn editor_history(&self) -> Vec<String> {
        self.load::<Vec<String>>(KEY_EDITOR_HISTORY)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_root(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("xyl-storage-{tag}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn segment_validation_rejects_escape() {
        assert!(valid_segment("default"));
        assert!(valid_segment("dev.1_2-x"));
        assert!(!valid_segment(""));
        assert!(!valid_segment("../escape"));
        assert!(!valid_segment("a/b"));
        assert!(!valid_segment("-lead"));
        assert!(ClientStorage::new("/tmp/x", "../bad").is_err());
    }

    #[test]
    fn update_roundtrip_and_lock_sidecar_and_tmp_cleanup() {
        let storage = ClientStorage::new(tmp_root("rt"), "default").unwrap();
        assert!(storage.load::<Vec<String>>(KEY_EDITOR_HISTORY).is_none());
        let v = storage
            .update::<Vec<String>, _>(KEY_EDITOR_HISTORY, Vec::new(), |v| {
                v.insert(0, "hello".into());
            })
            .unwrap();
        assert_eq!(v, vec!["hello".to_string()]);
        assert_eq!(storage.editor_history(), vec!["hello".to_string()]);
        let entries: Vec<String> = fs::read_dir(storage.dir())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(entries.iter().any(|e| e == "editor-history.json"));
        assert!(entries.iter().any(|e| e == "editor-history.lock"));
        assert!(!entries.iter().any(|e| e.ends_with(".tmp")));
    }

    #[test]
    fn corrupt_file_falls_back_to_none() {
        let storage = ClientStorage::new(tmp_root("corrupt"), "default").unwrap();
        fs::create_dir_all(storage.dir()).unwrap();
        let path = storage.key_path(KEY_EDITOR_HISTORY).unwrap();
        fs::write(&path, "{not json").unwrap();
        assert!(storage.load::<Vec<String>>(KEY_EDITOR_HISTORY).is_none());
    }

    #[test]
    fn update_failure_cleans_up_tmp() {
        let storage = ClientStorage::new(tmp_root("tmpclean"), "default").unwrap();
        fs::create_dir_all(storage.dir()).unwrap();
        let target = storage.key_path(KEY_EDITOR_HISTORY).unwrap();
        fs::create_dir(&target).unwrap();
        let _err = storage
            .update::<Vec<String>, _>(KEY_EDITOR_HISTORY, Vec::new(), |v| {
                v.insert(0, "x".into());
            })
            .unwrap_err();
        let leftovers: Vec<String> = fs::read_dir(storage.dir())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "失败路径不得残留 .tmp: {leftovers:?}");
    }

    #[test]
    fn channels_are_partitioned() {
        let root = tmp_root("chan");
        let a = ClientStorage::new(root.clone(), "default").unwrap();
        let b = ClientStorage::new(root, "dev").unwrap();
        a.record_editor_prompt("from-default");
        b.record_editor_prompt("from-dev");
        assert_eq!(a.editor_history(), vec!["from-default".to_string()]);
        assert_eq!(b.editor_history(), vec!["from-dev".to_string()]);
    }

    #[test]
    fn record_dedupes_and_caps() {
        let storage = ClientStorage::new(tmp_root("cap"), "default").unwrap();
        for i in 0..(EDITOR_HISTORY_CAP + 5) {
            storage.record_editor_prompt(&format!("p{i}"));
        }
        storage.record_editor_prompt("p3");
        let v = storage.editor_history();
        assert_eq!(v.len(), EDITOR_HISTORY_CAP);
        assert_eq!(v[0], "p3");
    }

    #[test]
    fn concurrent_updates_serialized_by_lock() {
        let storage = std::sync::Arc::new(ClientStorage::new(tmp_root("conc"), "default").unwrap());
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let s = storage.clone();
                std::thread::spawn(move || {
                    for j in 0..10 {
                        let text = format!("t{i}-{j}");
                        let _ = s.update::<Vec<String>, _>(KEY_EDITOR_HISTORY, Vec::new(), |v| {
                            v.insert(0, text)
                        });
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        let v = storage.editor_history();
        assert_eq!(v.len(), 80, "并发写须全数落地: {}", v.len());
    }
}
