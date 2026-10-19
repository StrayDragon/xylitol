//! Client-local storage channel (c2470): multi-instance shared surface state.
//!
//! 磁盘为真值 + 写路径 flock 串行化（读→改→临时文件原子 rename）；外部变更
//! 经使用点懒并盘感知（c2915：↑ 召回前重读，无常驻 watcher）。路径按 channel
//! 分桶：`<root>/<channel>/tui/<key>.json`，一 key 一文件；同 channel 多实例
//! 天然共享，跨 channel（dev / default）互不污染。只承载 client-local 态
//! （偏好 / 布局 / 历史）；会话数据归 Host，渲染包 `xylitol-tui` 不感知本
//! 模块（app 层接线）。

use std::fs;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

/// channel / key 段约束（防路径逃逸；首字符字母数字，其余字母数字与 . _ -）。
fn valid_segment(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// 编辑器 ↑/↓ 发送历史 key（首个迁移条目；cap 与进程内 editor 一致）。
pub const KEY_EDITOR_HISTORY: &str = "editor-history";

/// 编辑器历史上限（对齐包内 editor `add_to_history` 截断）。
const EDITOR_HISTORY_CAP: usize = 100;

/// client-local 存储通道句柄（进程内共享 `Arc`）。
#[derive(Debug, Clone)]
pub struct ClientStorage {
    /// `<root>/<channel>/tui`。
    dir: PathBuf,
}

impl ClientStorage {
    /// 环境派生：根 `XYLITOL_STATE_ROOT`（缺省 `~/.xylitol/state`），通道
    /// `XYLITOL_CHANNEL`（缺省 `default`；dev 并行调试设 `dev`）。非法段回落缺省。
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

    /// 显式构造；`channel` 须为合法段。
    pub fn new(root: impl Into<PathBuf>, channel: &str) -> Result<Self, String> {
        if !valid_segment(channel) {
            return Err(format!("invalid channel segment: {channel}"));
        }
        Ok(Self {
            dir: root.into().join(channel).join("tui"),
        })
    }

    /// 存储目录（测试 / 诊断用）。
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn key_path(&self, key: &str) -> Result<PathBuf, String> {
        if !valid_segment(key) {
            return Err(format!("invalid key segment: {key}"));
        }
        Ok(self.dir.join(format!("{key}.json")))
    }

    /// 读；缺失 / 损坏 / 非法 key → `None`（调用方回退 initial，最坏读到旧值）。
    pub fn load<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let path = self.key_path(key).ok()?;
        let raw = fs::read_to_string(path).ok()?;
        serde_json::from_str(&raw).ok()
    }

    /// 串行化更新：目录内独占锁下 读盘 → mutate → 临时文件原子 rename。
    /// 返回落盘后的最终值；锁等待为阻塞式（写临界区仅读盘 + rename，毫秒级）。
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
                // 失败路径清理半成品，不残留 .tmp（r2899 落盘语义）。
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

    /// 编辑器 ↑/↓ 历史：跨进程记录（去重置顶、cap 截断）。失败静默
    /// （best-effort；最坏丢一条本地历史）。
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

    /// 编辑器 ↑/↓ 历史：读盘（缺失 / 损坏 → 空）。
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
        // key 落点被同名目录占位 → rename 失败，走失败清理路径。
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
