//! Project trust store — persists trust decisions for project directories.
//!
//! Aligns with pi's trust-manager.ts. Manages a JSON trust file
//! (`~/.xylitol/trust.json`) that maps canonical directory paths to
//! boolean trust decisions. Supports directory-walk ancestor lookup and
//! file-based locking for concurrent access safety.

#![allow(dead_code)]
use std::collections::BTreeMap;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

// ── Types ───────────────────────────────────────────────────────────

/// A project trust decision: Some(true) = trusted, Some(false) = not trusted, None = no decision.
pub(crate) type TrustDecision = Option<bool>;

/// An entry in the trust store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrustStoreEntry {
    pub(crate) path: String,
    pub(crate) decision: bool,
}

/// An update to apply to the trust store.
#[derive(Debug, Clone)]
pub(crate) struct TrustUpdate {
    pub(crate) path: String,
    pub(crate) decision: TrustDecision,
}

/// A trust option presented to the user.
#[derive(Debug, Clone)]
pub(crate) struct TrustOption {
    pub(crate) label: String,
    pub(crate) trusted: bool,
    pub(crate) updates: Vec<TrustUpdate>,
    pub(crate) saved_path: Option<String>,
}

// ── Path utilities ──────────────────────────────────────────────────

/// Normalize a CWD path to its canonical absolute form.
fn normalize_cwd(cwd: &str) -> String {
    match std::fs::canonicalize(cwd) {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => {
            // Fallback: resolve relative to current dir
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

// ── Trust options ───────────────────────────────────────────────────

/// Build the list of trust options for a given project directory.
///
/// Options:
/// 1. Trust this project
/// 2. Trust parent folder (if there is one)
/// 3. Do not trust
pub(crate) fn get_project_trust_options(cwd: &str) -> Vec<TrustOption> {
    let trust_path = normalize_cwd(cwd);
    let parent_path = get_project_trust_parent_path(&trust_path);

    let mut options = vec![TrustOption {
        label: "Trust".to_string(),
        trusted: true,
        updates: vec![TrustUpdate {
            path: trust_path.clone(),
            decision: Some(true),
        }],
        saved_path: Some(trust_path.clone()),
    }];

    if let Some(ref parent) = parent_path {
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

    options.push(TrustOption {
        label: "Do not trust".to_string(),
        trusted: false,
        updates: vec![TrustUpdate {
            path: trust_path.clone(),
            decision: Some(false),
        }],
        saved_path: Some(trust_path),
    });

    options
}

/// Get the trust path for a CWD (canonicalized).
pub(crate) fn get_project_trust_path(cwd: &str) -> String {
    normalize_cwd(cwd)
}

/// Get the parent directory of the trust path, if one exists (not root).
pub(crate) fn get_project_trust_parent_path(trust_path: &str) -> Option<String> {
    let p = Path::new(trust_path);
    let parent = p.parent()?;
    if parent == p {
        return None;
    }
    Some(parent.to_string_lossy().to_string())
}

// ── Project trust inputs detection ──────────────────────────────────

/// Config directory name used across the project.
const CONFIG_DIR_NAME: &str = ".xylitol";

/// Check if the given directory has a `.xylitol/` config directory.
pub(crate) fn has_project_config_dir(cwd: &str) -> bool {
    let canonical = match std::fs::canonicalize(cwd) {
        Ok(p) => p,
        Err(_) => return false,
    };
    canonical.join(CONFIG_DIR_NAME).is_dir()
}

/// Check if a project has trust-relevant inputs (config dir, skills dir).
///
/// Walks up from CWD looking for `.xylitol/` or `.agents/skills/`.
pub(crate) fn has_project_trust_inputs(cwd: &str) -> bool {
    let mut current = match std::fs::canonicalize(cwd) {
        Ok(p) => p,
        Err(_) => return false,
    };

    // Check current dir first for .xylitol/
    if current.join(CONFIG_DIR_NAME).is_dir() {
        return true;
    }

    // Walk up looking for .agents/skills/
    loop {
        if current.join(".agents").join("skills").is_dir() {
            return true;
        }

        match current.parent() {
            Some(parent) if parent != current => {
                current = parent.to_path_buf();
            }
            _ => break,
        }
    }

    false
}

// ── Trust file I/O ──────────────────────────────────────────────────

/// Type alias for the in-memory representation of the trust file.
type TrustFile = BTreeMap<String, Option<bool>>;

/// Read and parse the trust JSON file.
fn read_trust_file(path: &Path) -> Result<TrustFile, String> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }

    let raw = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read trust store {path:?}: {e}"))?;

    let parsed: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("Invalid trust store {path:?}: {e}"))?;

    let obj = parsed
        .as_object()
        .ok_or_else(|| format!("Invalid trust store {path:?}: expected an object"))?;

    let mut data = TrustFile::new();
    for (key, value) in obj {
        match value {
            serde_json::Value::Bool(b) => {
                data.insert(key.clone(), Some(*b));
            }
            serde_json::Value::Null => {
                data.insert(key.clone(), None);
            }
            other => {
                return Err(format!(
                    "Invalid trust store {path:?}: value for {key:?} must be true, false, or null, got {other}"
                ));
            }
        }
    }
    Ok(data)
}

/// Write the trust data to the JSON file.
fn write_trust_file(path: &Path, data: &TrustFile) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create trust dir {parent:?}: {e}"))?;
    }

    // Filter to valid entries and serialize
    let filtered: BTreeMap<&String, &Option<bool>> = data.iter().collect();
    let json = serde_json::to_string_pretty(&filtered)
        .map_err(|e| format!("Failed to serialize trust data: {e}"))?;

    fs::write(path, format!("{json}\n"))
        .map_err(|e| format!("Failed to write trust store {path:?}: {e}"))?;

    Ok(())
}

// ── File locking ────────────────────────────────────────────────────

/// Acquire an exclusive lock on the trust file using a lock file + retry.
///
/// Uses `O_CREAT | O_EXCL` on a `.lock` file. Retries up to 10 times
/// with 20ms delays.
fn acquire_trust_lock(path: &Path) -> Result<fs::File, String> {
    let lock_path = path.with_extension("json.lock");

    // Ensure parent dir exists
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create trust lock dir {parent:?}: {e}"))?;
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
                // Write PID for debugging
                let _ = (&file).write_all(format!("{}\n", std::process::id()).as_bytes());
                return Ok(file);
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                if attempt == max_attempts {
                    return Err(format!(
                        "Failed to acquire trust store lock after {max_attempts} attempts: {e}"
                    ));
                }
                std::thread::sleep(delay);
            }
            Err(e) => {
                return Err(format!(
                    "Failed to create trust lock file {lock_path:?}: {e}"
                ));
            }
        }
    }

    Err("Failed to acquire trust store lock".to_string())
}

/// Release the lock by removing the lock file.
fn release_trust_lock(path: &Path, _lock_file: fs::File) {
    let lock_path = path.with_extension("json.lock");
    // Dropping the file handle closes it; then remove the lock file.
    drop(_lock_file);
    let _ = fs::remove_file(&lock_path);
}

/// Execute a function with the trust file locked.
fn with_trust_file_lock<T>(
    path: &Path,
    f: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let lock = acquire_trust_lock(path)?;
    let result = f();
    release_trust_lock(path, lock);
    result
}

// ── TrustStore ──────────────────────────────────────────────────────

/// Persistent store for project trust decisions.
///
/// Reads/writes a `trust.json` file in the agent config directory.
#[derive(Debug, Clone)]
pub(crate) struct TrustStore {
    trust_path: PathBuf,
}

impl TrustStore {
    /// Create a new TrustStore.
    ///
    /// `agent_dir` should be the global config directory (e.g. `~/.xylitol/`).
    pub(crate) fn new(agent_dir: &Path) -> Self {
        Self {
            trust_path: agent_dir.join("trust.json"),
        }
    }

    /// Get the trust decision for a given CWD.
    ///
    /// Walks up the directory tree to find the nearest ancestor with a stored decision.
    /// Returns `None` if no decision is found.
    pub(crate) fn get(&self, cwd: &str) -> TrustDecision {
        self.get_entry(cwd).map(|e| e.decision)
    }

    /// Get the nearest trust entry for a CWD (with path info).
    pub(crate) fn get_entry(&self, cwd: &str) -> Option<TrustStoreEntry> {
        with_trust_file_lock(&self.trust_path, || {
            let data = read_trust_file(&self.trust_path)?;
            Ok(find_nearest_trust_entry(&data, cwd))
        })
        .unwrap_or(None)
    }

    /// Set a single trust decision for a CWD.
    pub(crate) fn set(&self, cwd: &str, decision: TrustDecision) {
        self.set_many(&[TrustUpdate {
            path: cwd.to_string(),
            decision,
        }]);
    }

    /// Apply multiple trust updates atomically.
    pub(crate) fn set_many(&self, decisions: &[TrustUpdate]) {
        let _ = with_trust_file_lock(&self.trust_path, || {
            let mut data = read_trust_file(&self.trust_path)?;
            for update in decisions {
                let key = normalize_cwd(&update.path);
                match update.decision {
                    Some(b) => {
                        data.insert(key, Some(b));
                    }
                    None => {
                        data.remove(&key);
                    }
                }
            }
            write_trust_file(&self.trust_path, &data)
        });
    }

    /// Returns the path to the trust file (for testing/debugging).
    pub(crate) fn trust_file_path(&self) -> &Path {
        &self.trust_path
    }
}

impl Default for TrustStore {
    fn default() -> Self {
        let agent_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".xylitol");
        Self::new(&agent_dir)
    }
}

// ── Nearest trust entry lookup ──────────────────────────────────────

/// Walk up the directory tree to find the nearest ancestor with a stored trust decision.
fn find_nearest_trust_entry(data: &TrustFile, cwd: &str) -> Option<TrustStoreEntry> {
    let mut current = normalize_cwd(cwd);

    loop {
        if let Some(&Some(decision)) = data.get(&current) {
            return Some(TrustStoreEntry {
                path: current,
                decision,
            });
        }

        let parent = Path::new(&current).parent()?;
        if parent == Path::new(&current) {
            return None;
        }
        current = parent.to_string_lossy().to_string();
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_cwd_current_dir() {
        let cwd = std::env::current_dir().unwrap();
        let normalized = normalize_cwd(".");
        assert_eq!(normalized, cwd.to_string_lossy().to_string());
    }

    #[test]
    fn test_trust_store_set_and_get() {
        let tmp = tempfile::tempdir().unwrap();
        let store = TrustStore::new(tmp.path());

        // Initially no decision
        assert_eq!(store.get("/some/project"), None);

        // Set a decision
        store.set("/some/project", Some(true));

        // But canonicalize resolves, so this may not work exactly as expected
        // Let's test with the canonical path
        let proj_path = tmp.path().join("my-project");
        fs::create_dir_all(&proj_path).unwrap();
        let canonical = fs::canonicalize(&proj_path).unwrap();
        let canonical_str = canonical.to_string_lossy().to_string();

        store.set(&canonical_str, Some(true));
        assert_eq!(store.get(&canonical_str), Some(true));
    }

    #[test]
    fn test_trust_store_remove() {
        let tmp = tempfile::tempdir().unwrap();
        let store = TrustStore::new(tmp.path());

        let proj = tmp.path().join("proj");
        fs::create_dir_all(&proj).unwrap();
        let canonical = fs::canonicalize(&proj)
            .unwrap()
            .to_string_lossy()
            .to_string();

        store.set(&canonical, Some(true));
        assert_eq!(store.get(&canonical), Some(true));

        store.set(&canonical, None);
        assert_eq!(store.get(&canonical), None);
    }

    #[test]
    fn test_get_project_trust_options() {
        let cwd = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let options = get_project_trust_options(&cwd);

        // Always at least "Trust" and "Do not trust"
        assert!(options.len() >= 2);
        assert_eq!(options[0].label, "Trust");
        assert!(options[0].trusted);
        let last = options.last().unwrap();
        assert_eq!(last.label, "Do not trust");
        assert!(!last.trusted);
    }

    #[test]
    fn test_has_project_config_dir() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!has_project_config_dir(&tmp.path().to_string_lossy()));

        fs::create_dir_all(tmp.path().join(".xylitol")).unwrap();
        assert!(has_project_config_dir(&tmp.path().to_string_lossy()));
    }

    #[test]
    fn test_has_project_trust_inputs_no_inputs() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!has_project_trust_inputs(&tmp.path().to_string_lossy()));
    }

    #[test]
    fn test_has_project_trust_inputs_with_config() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".xylitol")).unwrap();
        assert!(has_project_trust_inputs(&tmp.path().to_string_lossy()));
    }

    #[test]
    fn test_has_project_trust_inputs_with_skills() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".agents").join("skills")).unwrap();
        assert!(has_project_trust_inputs(&tmp.path().to_string_lossy()));
    }

    #[test]
    fn test_trust_file_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("trust.json");

        let mut data = TrustFile::new();
        data.insert("/a/b".to_string(), Some(true));
        data.insert("/a/c".to_string(), Some(false));
        data.insert("/a/d".to_string(), None);

        write_trust_file(&path, &data).unwrap();
        let read_back = read_trust_file(&path).unwrap();

        assert_eq!(read_back.get("/a/b"), Some(&Some(true)));
        assert_eq!(read_back.get("/a/c"), Some(&Some(false)));
        assert_eq!(read_back.get("/a/d"), Some(&None));
    }

    #[test]
    fn test_read_nonexistent_trust_file() {
        let result = read_trust_file(Path::new("/nonexistent/path/trust.json"));
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
