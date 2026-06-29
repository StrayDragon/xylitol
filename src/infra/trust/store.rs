//! Project trust store — persistence, locking, and inheritance (spec c255 / t1, t2).
//!
//! Single source of truth for project trust decisions. Persists a pi-style
//! JSON file mapping canonical absolute directory paths to a boolean decision
//! (`true`=trusted, `false`=untrusted, `null`=cleared). Supports
//! parent-directory inheritance and exclusive file locking for concurrent-safe
//! access.

use std::collections::BTreeMap;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::runtime_protocol::XyTrustStore;

/// In-memory trust file: canonical path → decision. Sorted for deterministic output.
type TrustFile = BTreeMap<String, Option<bool>>;

/// A trust decision: `Some(true)`=trusted, `Some(false)`=untrusted, `None`=cleared.
pub type TrustDecision = Option<bool>;

/// A trust option presented to the user (carries the persisted updates).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustOption {
    pub label: String,
    pub trusted: bool,
    pub updates: Vec<TrustUpdate>,
    pub saved_path: Option<String>,
}

/// A single trust store update entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustUpdate {
    pub path: String,
    pub decision: TrustDecision,
}

/// Config directory name used across the project.
const CONFIG_DIR_NAME: &str = ".xylitol";

/// Manager for project trust decisions — the single source of truth (spec t1).
#[derive(Debug, Clone)]
pub struct TrustManager {
    trust_file_path: PathBuf,
}

impl TrustManager {
    /// Create a TrustManager whose store lives at `trust_dir/trust.json`.
    pub fn new(trust_dir: impl Into<PathBuf>) -> Self {
        let mut path = trust_dir.into();
        fs::create_dir_all(&path).ok();
        path.push("trust.json");
        Self {
            trust_file_path: path,
        }
    }

    /// Default trust store location: `~/.xylitol/trust.json`.
    pub fn default_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(CONFIG_DIR_NAME)
    }

    /// Path to the underlying trust file.
    pub fn trust_file_path(&self) -> &Path {
        &self.trust_file_path
    }

    // ── Path normalization ───────────────────────────────────────

    /// Canonicalize a CWD, falling back to the literal path when it cannot be resolved.
    fn normalize_cwd(cwd: &str) -> String {
        match fs::canonicalize(cwd) {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(_) => {
                let p = Path::new(cwd);
                if p.is_absolute() {
                    p.to_string_lossy().to_string()
                } else {
                    std::env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .join(p)
                        .to_string_lossy()
                        .to_string()
                }
            }
        }
    }

    // ── File locking (migrated from the former agent/trust/store.rs) ──

    /// Acquire an exclusive lock via a `.lock` sibling using `O_CREAT | O_EXCL`,
    /// retrying briefly to tolerate concurrent writers.
    fn acquire_lock(&self) -> Result<fs::File, String> {
        let lock_path = self.trust_file_path.with_extension("json.lock");
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("create trust lock dir {parent:?}: {e}"))?;
        }
        let max_attempts = 10;
        let delay = Duration::from_millis(20);
        for attempt in 1..=max_attempts {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(file) => {
                    let _ = (&file).write_all(format!("{}\n", std::process::id()).as_bytes());
                    return Ok(file);
                }
                Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                    if attempt == max_attempts {
                        return Err(format!(
                            "trust store lock contention after {max_attempts} attempts: {e}"
                        ));
                    }
                    std::thread::sleep(delay);
                }
                Err(e) => {
                    return Err(format!("create trust lock {lock_path:?}: {e}"));
                }
            }
        }
        Err("failed to acquire trust store lock".to_string())
    }

    fn release_lock(&self, lock_file: fs::File) {
        let lock_path = self.trust_file_path.with_extension("json.lock");
        drop(lock_file);
        let _ = fs::remove_file(&lock_path);
    }

    fn with_lock<T>(&self, f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
        let lock = self.acquire_lock()?;
        let result = f();
        self.release_lock(lock);
        result
    }

    // ── File I/O ─────────────────────────────────────────────────

    fn read_file(&self) -> Result<TrustFile, String> {
        if !self.trust_file_path.exists() {
            return Ok(BTreeMap::new());
        }
        let raw = fs::read_to_string(&self.trust_file_path).map_err(|e| {
            format!(
                "read trust store {path:?}: {e}",
                path = self.trust_file_path
            )
        })?;
        let parsed: TrustFile = serde_json::from_str(&raw).map_err(|e| {
            format!(
                "parse trust store {path:?}: {e}",
                path = self.trust_file_path
            )
        })?;
        Ok(parsed)
    }

    fn write_file(&self, data: &TrustFile) -> Result<(), String> {
        if let Some(parent) = self.trust_file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("create trust dir {parent:?}: {e}"))?;
        }
        // BTreeMap yields sorted keys → deterministic output.
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| format!("serialize trust store: {e}"))?;
        // Atomic write: tmp file then rename.
        let tmp_path = self.trust_file_path.with_extension("json.tmp");
        fs::write(&tmp_path, format!("{json}\n"))
            .map_err(|e| format!("write trust tmp {tmp_path:?}: {e}"))?;
        fs::rename(&tmp_path, &self.trust_file_path).map_err(|e| format!("rename trust store: {e}"))
    }

    // ── Public API ───────────────────────────────────────────────

    /// Check whether a directory is trusted, walking up the tree to find the
    /// nearest stored ancestor decision (parent inheritance, spec t2).
    /// Returns `None` when no decision is found (undecided).
    pub fn is_project_trusted(&self, cwd: &str) -> TrustDecision {
        self.with_lock(|| {
            let data = self.read_file()?;
            Ok(find_nearest_decision(&data, cwd))
        })
        .unwrap_or(None)
    }

    /// Shorthand for `is_project_trusted(cwd) == Some(true)`.
    pub fn is_trusted(&self, cwd: &str) -> bool {
        self.is_project_trusted(cwd) == Some(true)
    }

    /// Persist a single trust decision for a path.
    pub fn set_trust(&self, path: &str, decision: TrustDecision) -> Result<(), String> {
        self.apply_updates(&[TrustUpdate {
            path: path.to_string(),
            decision,
        }])
    }

    /// Apply multiple trust updates atomically under the lock.
    pub fn apply_updates(&self, updates: &[TrustUpdate]) -> Result<(), String> {
        self.with_lock(|| {
            let mut data = self.read_file()?;
            for update in updates {
                let key = Self::normalize_cwd(&update.path);
                match update.decision {
                    Some(b) => {
                        data.insert(key, Some(b));
                    }
                    None => {
                        data.remove(&key);
                    }
                }
            }
            self.write_file(&data)
        })
    }

    /// Build the list of trust options for a project directory.
    ///
    /// Without session-only: [Trust, Trust parent (if any), Do not trust].
    /// With session-only: appends non-persisting "this session only" variants.
    pub fn get_trust_options(&self, cwd: &str, include_session_only: bool) -> Vec<TrustOption> {
        let trust_path = Self::normalize_cwd(cwd);
        let parent_path = parent_dir(&trust_path);

        let mut options = vec![TrustOption {
            label: "Trust".into(),
            trusted: true,
            updates: vec![TrustUpdate {
                path: trust_path.clone(),
                decision: Some(true),
            }],
            saved_path: Some(trust_path.clone()),
        }];

        if let Some(ref parent) = parent_path
            && *parent != trust_path
        {
            options.push(TrustOption {
                label: format!("Trust parent folder ({parent})"),
                trusted: true,
                updates: vec![
                    TrustUpdate {
                        path: parent.clone(),
                        decision: Some(true),
                    },
                    TrustUpdate {
                        path: trust_path.clone(),
                        decision: None,
                    },
                ],
                saved_path: Some(parent.clone()),
            });
        }

        if include_session_only {
            options.push(TrustOption {
                label: "Trust (this session only)".into(),
                trusted: true,
                updates: vec![],
                saved_path: None,
            });
        }

        options.push(TrustOption {
            label: "Do not trust".into(),
            trusted: false,
            updates: vec![TrustUpdate {
                path: trust_path.clone(),
                decision: Some(false),
            }],
            saved_path: Some(trust_path),
        });

        if include_session_only {
            options.push(TrustOption {
                label: "Do not trust (this session only)".into(),
                trusted: false,
                updates: vec![],
                saved_path: None,
            });
        }

        options
    }

    /// Detect whether a directory has trust-requiring inputs: a local
    /// `.xylitol/` config dir, or an ancestor `.agents/skills/` dir.
    /// Used by the resolver to decide the no-inputs auto-trust step (spec t3).
    pub fn has_trust_inputs(&self, cwd: &str) -> bool {
        let mut current = match fs::canonicalize(cwd) {
            Ok(p) => p,
            Err(_) => return false,
        };
        if current.join(CONFIG_DIR_NAME).is_dir() {
            return true;
        }
        loop {
            if current.join(".agents").join("skills").is_dir() {
                return true;
            }
            match current.parent() {
                Some(parent) if parent != current => current = parent.to_path_buf(),
                _ => break,
            }
        }
        false
    }
}

impl XyTrustStore for TrustManager {
    fn set_trust(&self, path: &str, trusted: Option<bool>) -> Result<(), String> {
        self.set_trust(path, trusted)
    }
}

// ── Free helpers ───────────────────────────────────────────────────

/// Walk up the directory tree to find the nearest ancestor with a stored decision.
fn find_nearest_decision(data: &TrustFile, cwd: &str) -> TrustDecision {
    let mut current = TrustManager::normalize_cwd(cwd);
    loop {
        if let Some(decision) = data.get(&current) {
            return *decision;
        }
        let parent = parent_dir(&current)?;
        if parent == current {
            return None;
        }
        current = parent;
    }
}

/// Parent directory of a path string (canonical, non-root), or `None`.
fn parent_dir(path_str: &str) -> Option<String> {
    let p = Path::new(path_str);
    let parent = p.parent()?;
    if parent == p {
        return None;
    }
    Some(parent.to_string_lossy().to_string())
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (TrustManager, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("create temp dir");
        let mgr = TrustManager::new(dir.path().to_path_buf());
        (mgr, dir)
    }

    #[test]
    fn test_default_is_undecided() {
        let (mgr, _dir) = setup();
        assert_eq!(mgr.is_project_trusted("/some/project"), None);
    }

    #[test]
    fn test_set_and_check_trust() {
        let (mgr, dir) = setup();
        let path = dir.path().join("my-project");
        fs::create_dir_all(&path).unwrap();
        let path_str = path.to_string_lossy().to_string();

        mgr.set_trust(&path_str, Some(true)).unwrap();
        assert_eq!(mgr.is_project_trusted(&path_str), Some(true));
    }

    #[test]
    fn test_set_untrusted() {
        let (mgr, dir) = setup();
        let path = dir.path().join("untrusted-project");
        fs::create_dir_all(&path).unwrap();
        let path_str = path.to_string_lossy().to_string();

        mgr.set_trust(&path_str, Some(false)).unwrap();
        assert_eq!(mgr.is_project_trusted(&path_str), Some(false));
    }

    #[test]
    fn test_parent_inheritance() {
        let (mgr, dir) = setup();
        let parent = dir.path().join("parent");
        let child = parent.join("child");
        fs::create_dir_all(&child).unwrap();

        mgr.set_trust(&parent.to_string_lossy(), Some(true))
            .unwrap();
        assert_eq!(mgr.is_project_trusted(&child.to_string_lossy()), Some(true));
    }

    #[test]
    fn test_child_override_nearest_ancestor_wins() {
        // t2 child-override: nearest stored decision wins over a more distant ancestor.
        let (mgr, dir) = setup();
        let parent = dir.path().join("parent");
        let child = parent.join("child");
        fs::create_dir_all(&child).unwrap();

        mgr.set_trust(&parent.to_string_lossy(), Some(true))
            .unwrap();
        mgr.set_trust(&child.to_string_lossy(), Some(false))
            .unwrap();
        assert_eq!(
            mgr.is_project_trusted(&child.to_string_lossy()),
            Some(false)
        );
        // Sibling without its own decision still inherits the parent's.
        let sibling = parent.join("sibling");
        fs::create_dir_all(&sibling).unwrap();
        assert_eq!(
            mgr.is_project_trusted(&sibling.to_string_lossy()),
            Some(true)
        );
    }

    #[test]
    fn test_clear_trust() {
        let (mgr, dir) = setup();
        let path = dir.path().join("cleared");
        fs::create_dir_all(&path).unwrap();
        let path_str = path.to_string_lossy().to_string();

        mgr.set_trust(&path_str, Some(true)).unwrap();
        mgr.set_trust(&path_str, None).unwrap();
        assert_eq!(mgr.is_project_trusted(&path_str), None);
    }

    #[test]
    fn test_apply_updates_multiple() {
        let (mgr, dir) = setup();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        let a_str = a.to_string_lossy().to_string();
        let b_str = b.to_string_lossy().to_string();

        mgr.apply_updates(&[
            TrustUpdate {
                path: a_str.clone(),
                decision: Some(true),
            },
            TrustUpdate {
                path: b_str.clone(),
                decision: Some(false),
            },
        ])
        .unwrap();
        assert_eq!(mgr.is_project_trusted(&a_str), Some(true));
        assert_eq!(mgr.is_project_trusted(&b_str), Some(false));
    }

    #[test]
    fn test_get_trust_options() {
        let (mgr, dir) = setup();
        let cwd = dir.path().join("project");
        fs::create_dir_all(&cwd).unwrap();
        let cwd_str = cwd.to_string_lossy().to_string();

        let options = mgr.get_trust_options(&cwd_str, false);
        assert!(options.len() >= 2);
        assert!(options.iter().any(|o| o.label == "Trust"));
        assert!(options.iter().any(|o| o.label.contains("Do not trust")));
    }

    #[test]
    fn test_get_trust_options_with_session() {
        let (mgr, dir) = setup();
        let cwd = dir.path().join("project");
        fs::create_dir_all(&cwd).unwrap();
        let cwd_str = cwd.to_string_lossy().to_string();

        let options = mgr.get_trust_options(&cwd_str, true);
        let session_only = options
            .iter()
            .filter(|o| o.label.contains("this session only"))
            .count();
        assert_eq!(session_only, 2, "expected two session-only variants");
        assert_eq!(
            mgr.get_trust_options(&cwd_str, false).len() + 2,
            options.len()
        );
    }

    #[test]
    fn test_has_trust_inputs_no_inputs() {
        let (mgr, dir) = setup();
        assert!(!mgr.has_trust_inputs(&dir.path().to_string_lossy()));
    }

    #[test]
    fn test_has_trust_inputs_with_config_dir() {
        let (mgr, dir) = setup();
        fs::create_dir_all(dir.path().join(".xylitol")).unwrap();
        assert!(mgr.has_trust_inputs(&dir.path().to_string_lossy()));
    }

    #[test]
    fn test_has_trust_inputs_with_skills_ancestor() {
        let (mgr, dir) = setup();
        let child = dir.path().join("deep").join("project");
        fs::create_dir_all(&child).unwrap();
        fs::create_dir_all(dir.path().join(".agents").join("skills")).unwrap();
        assert!(mgr.has_trust_inputs(&child.to_string_lossy()));
    }

    #[test]
    fn test_persistence_format_pi_style() {
        // t2: verify the on-disk JSON is the pi-style {path: bool|null} format.
        let (mgr, dir) = setup();
        let path = dir.path().join("proj");
        fs::create_dir_all(&path).unwrap();
        mgr.set_trust(&path.to_string_lossy(), Some(true)).unwrap();

        let on_disk = fs::read_to_string(mgr.trust_file_path()).expect("read trust file");
        assert!(
            on_disk.contains(": true") || on_disk.contains(":true"),
            "expected boolean decision in JSON, got: {on_disk}"
        );
    }
}
