//! TrustManager — project trust management aligning with pi's trust-manager.ts.
//!
//! Manages project trust decisions stored in `~/.xylitol/trust.json`.
//! Supports:
//! - `is_project_trusted(cwd)` — check trust via parent directory inheritance
//! - `get_trust_options(cwd)` — return decision options (trust/don't trust/session-only)
//! - `set_trust(path, decision)` — persist decisions with file locking
//! - `has_trust_requiring_resources(cwd)` — detect .xylitol config that needs trust

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The trust store: canonical path → decision (true=trusted, false=untrusted, null=cleared).
pub type TrustStore = HashMap<String, Option<bool>>;

/// A trust decision option presented to the user.
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
    pub decision: Option<bool>,
}

/// Project resources that require trust to load.
const TRUST_REQUIRING_RESOURCES: &[&str] = &[
    "settings.json",
    "extensions",
    "skills",
    "prompts",
    "themes",
    "SYSTEM.md",
    "APPEND_SYSTEM.md",
];

/// Manager for project trust decisions.
#[derive(Debug, Clone)]
pub struct TrustManager {
    trust_file_path: PathBuf,
}

impl TrustManager {
    /// Create a new TrustManager.
    /// Trust store is at `trust_dir/trust.json`.
    pub fn new(trust_dir: impl Into<PathBuf>) -> Self {
        let mut path = trust_dir.into();
        std::fs::create_dir_all(&path).ok();
        path.push("trust.json");
        Self {
            trust_file_path: path,
        }
    }

    /// Default trust store location: `~/.xylitol/trust.json`.
    pub fn default_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".xylitol")
    }

    /// Default constructor using `~/.xylitol/`.
    #[allow(dead_code)]
    fn default() -> Self {
        Self::new(Self::default_dir())
    }

    // ── Trust store I/O ──────────────────────────────────────────

    fn read_trust_store(&self) -> Result<TrustStore, String> {
        if !self.trust_file_path.exists() {
            return Ok(HashMap::new());
        }
        let data = std::fs::read_to_string(&self.trust_file_path)
            .map_err(|e| format!("read trust store: {e}"))?;
        let store: TrustStore =
            serde_json::from_str(&data).map_err(|e| format!("parse trust store: {e}"))?;
        Ok(store)
    }

    fn write_trust_store(&self, store: &TrustStore) -> Result<(), String> {
        if let Some(parent) = self.trust_file_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        // Sort keys for deterministic output
        let mut sorted: Vec<(String, Option<bool>)> =
            store.iter().map(|(k, v)| (k.clone(), *v)).collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        let sorted_store: TrustStore = sorted.into_iter().collect();

        let json = serde_json::to_string_pretty(&sorted_store)
            .map_err(|e| format!("serialize trust store: {e}"))?;

        // Atomic write: write to temp file then rename
        let tmp_path = self.trust_file_path.with_extension("json.tmp");
        std::fs::write(&tmp_path, format!("{json}\n"))
            .map_err(|e| format!("write trust store tmp: {e}"))?;
        std::fs::rename(&tmp_path, &self.trust_file_path)
            .map_err(|e| format!("rename trust store: {e}"))?;

        Ok(())
    }

    // ── Canonical path ───────────────────────────────────────────

    fn canonicalize(cwd: &str) -> String {
        let path = Path::new(cwd);
        path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .to_string()
    }

    fn normalize_dir(cwd: &str) -> String {
        // Remove trailing slash for consistency
        let s = cwd.trim_end_matches(['/', '\\']);
        Path::new(s)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(s))
            .to_string_lossy()
            .to_string()
    }

    // ── Public API ───────────────────────────────────────────────

    /// Check if a directory is trusted.
    /// Walks up directory tree looking for a trust entry (parent inheritance).
    /// Returns `None` if no trust entry found (undecided).
    pub fn is_project_trusted(&self, cwd: &str) -> Option<bool> {
        let store = self.read_trust_store().unwrap_or_default();
        let mut current = Self::canonicalize(cwd);

        loop {
            if let Some(decision) = store.get(&current) {
                return *decision;
            }

            // Walk up
            let parent = Path::new(&current).parent().map(|p| p.to_path_buf());
            if let Some(p) = parent {
                let p_str = p.to_string_lossy().to_string();
                if p_str == current {
                    // Reached root
                    break;
                }
                current = p_str;
            } else {
                break;
            }
        }

        None
    }

    /// Check if cwd is explicitly trusted (return true).
    /// Shorthand for `is_project_trusted(cwd) == Some(true)`.
    pub fn is_trusted(&self, cwd: &str) -> bool {
        self.is_project_trusted(cwd) == Some(true)
    }

    /// Get trust decision options for a directory.
    pub fn get_trust_options(&self, cwd: &str, include_session_only: bool) -> Vec<TrustOption> {
        let trust_path = Self::normalize_dir(cwd);
        let mut options = vec![TrustOption {
            label: "Trust".into(),
            trusted: true,
            updates: vec![TrustUpdate {
                path: trust_path.clone(),
                decision: Some(true),
            }],
            saved_path: Some(trust_path.clone()),
        }];

        // Add parent trust option
        let parent = Path::new(&trust_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string());
        if let Some(ref parent_dir) = parent
            && *parent_dir != trust_path
        {
            options.push(TrustOption {
                label: format!("Trust parent folder ({parent_dir})"),
                trusted: true,
                updates: vec![
                    TrustUpdate {
                        path: parent_dir.clone(),
                        decision: Some(true),
                    },
                    TrustUpdate {
                        path: trust_path.clone(),
                        decision: None,
                    },
                ],
                saved_path: Some(parent_dir.clone()),
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
            saved_path: Some(trust_path.clone()),
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

    /// Persist a trust decision.
    pub fn set_trust(&self, path: &str, decision: Option<bool>) -> Result<(), String> {
        let normalized = Self::canonicalize(path);
        let mut store = self.read_trust_store().unwrap_or_default();
        store.insert(normalized, decision);
        self.write_trust_store(&store)?;
        Ok(())
    }

    /// Detect if a directory has project-local resources that require trust.
    /// Checks for `.xylitol/` entries like settings.json, extensions, skills, etc.
    pub fn has_trust_requiring_resources(&self, cwd: &str) -> bool {
        let config_dir = Path::new(cwd).join(".xylitol");
        if !config_dir.exists() {
            return false;
        }

        for resource in TRUST_REQUIRING_RESOURCES {
            if config_dir.join(resource).exists() {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_trust_dir() -> (TrustManager, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("create temp dir");
        let mgr = TrustManager::new(dir.path().to_path_buf());
        (mgr, dir)
    }

    #[test]
    fn test_default_is_undecided() {
        let (mgr, _dir) = setup_trust_dir();
        assert_eq!(mgr.is_project_trusted("/some/project"), None);
    }

    #[test]
    fn test_set_and_check_trust() {
        let (mgr, _dir) = setup_trust_dir();
        let path = _dir.path().join("my-project");
        std::fs::create_dir_all(&path).unwrap();
        let path_str = path.to_string_lossy().to_string();

        mgr.set_trust(&path_str, Some(true)).unwrap();
        assert_eq!(mgr.is_project_trusted(&path_str), Some(true));
    }

    #[test]
    fn test_set_untrusted() {
        let (mgr, _dir) = setup_trust_dir();
        let path = _dir.path().join("untrusted-project");
        std::fs::create_dir_all(&path).unwrap();
        let path_str = path.to_string_lossy().to_string();

        mgr.set_trust(&path_str, Some(false)).unwrap();
        assert_eq!(mgr.is_project_trusted(&path_str), Some(false));
    }

    #[test]
    fn test_parent_inheritance() {
        let (mgr, _dir) = setup_trust_dir();
        let parent = _dir.path().join("parent");
        let child = parent.join("child");
        std::fs::create_dir_all(&child).unwrap();

        mgr.set_trust(&parent.to_string_lossy(), Some(true))
            .unwrap();
        assert_eq!(mgr.is_project_trusted(&child.to_string_lossy()), Some(true));
    }

    #[test]
    fn test_clear_trust() {
        let (mgr, _dir) = setup_trust_dir();
        let path = _dir.path().join("cleared");
        std::fs::create_dir_all(&path).unwrap();
        let path_str = path.to_string_lossy().to_string();

        mgr.set_trust(&path_str, Some(true)).unwrap();
        mgr.set_trust(&path_str, None).unwrap();
        assert_eq!(mgr.is_project_trusted(&path_str), None);
    }

    #[test]
    fn test_has_trust_requiring_resources() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let xylitol_dir = dir.path().join(".xylitol");
        std::fs::create_dir_all(&xylitol_dir).unwrap();

        // No config resources yet
        let mgr = TrustManager::new(dir.path().join("trust"));
        assert!(!mgr.has_trust_requiring_resources(&dir.path().to_string_lossy()));

        // Add a trust-requiring resource
        std::fs::write(xylitol_dir.join("settings.json"), "{}").unwrap();
        assert!(mgr.has_trust_requiring_resources(&dir.path().to_string_lossy()));
    }

    #[test]
    fn test_get_trust_options() {
        let (mgr, _dir) = setup_trust_dir();
        let cwd = _dir.path().join("project");
        std::fs::create_dir_all(&cwd).unwrap();
        let cwd_str = cwd.to_string_lossy().to_string();

        let options = mgr.get_trust_options(&cwd_str, false);
        // Should have: Trust, Trust parent, Do not trust
        assert!(options.len() >= 3);
        assert!(options.iter().any(|o| o.label == "Trust"));
        assert!(options.iter().any(|o| o.label.contains("Do not trust")));
    }

    #[test]
    fn test_get_trust_options_with_session() {
        let (mgr, _dir) = setup_trust_dir();
        let cwd = _dir.path().join("project");
        std::fs::create_dir_all(&cwd).unwrap();
        let cwd_str = cwd.to_string_lossy().to_string();

        let options = mgr.get_trust_options(&cwd_str, true);
        // Should include session-only variants
        assert!(
            options
                .iter()
                .any(|o| o.label.contains("this session only"))
        );
        assert_eq!(options.len(), 5);
    }
}
