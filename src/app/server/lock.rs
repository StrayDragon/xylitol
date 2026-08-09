//! Single-instance lock — ensures at most one server process runs per lock path.
//!
//! Uses `O_CREAT | O_EXCL` atomic file creation (POSIX-only on Unix; on other
//! platforms we attempt the same but may race; the lock is advisory only).
//! The lock file is deleted on drop (graceful shutdown).

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Payload written into the lock file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInfo {
    pub port: u16,
    pub pid: u32,
    pub hostname: String,
}

/// Error returned when the lock cannot be acquired.
#[derive(Debug)]
pub enum ServerLockedError {
    /// Another server is already running (lock file exists and is valid).
    AlreadyRunning(LockInfo),
    /// An I/O error occurred (permissions, filesystem, etc.).
    Io(std::io::Error),
}

impl std::fmt::Display for ServerLockedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyRunning(info) => {
                write!(
                    f,
                    "server already running on port {} (pid {}, hostname {})",
                    info.port, info.pid, info.hostname
                )
            }
            Self::Io(e) => write!(f, "lock I/O error: {e}"),
        }
    }
}

impl std::error::Error for ServerLockedError {}

impl From<std::io::Error> for ServerLockedError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// The server lock handle. Held for the lifetime of the server process.
/// On `Drop`, the lock file is deleted (best-effort).
#[derive(Debug)]
pub struct ServerLock {
    lock_path: PathBuf,
    // File handle is held open to keep the lock alive.
    #[allow(dead_code)]
    handle: File,
}

impl ServerLock {
    /// Try to acquire the lock at `lock_path`.
    ///
    /// Creates the lock file atomically (`O_CREAT | O_EXCL`). If the file
    /// already exists, reads its content and returns [`ServerLockedError::AlreadyRunning`].
    pub fn try_acquire(lock_path: &Path, info: &LockInfo) -> Result<Self, ServerLockedError> {
        // Ensure parent directory exists.
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent).map_err(ServerLockedError::Io)?;
        }

        // Try O_CREAT | O_EXCL via OpenOptions.
        let handle = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(lock_path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    // Lock exists — read the existing info.
                    match Self::read_lock_info(lock_path) {
                        Ok(info) => ServerLockedError::AlreadyRunning(info),
                        Err(io_err) => ServerLockedError::Io(io_err),
                    }
                } else {
                    ServerLockedError::Io(e)
                }
            })?;

        // Write our lock info.
        let payload = serde_json::to_string(info).expect("LockInfo serialization never fails");
        write!(&handle, "{payload}").map_err(ServerLockedError::Io)?;

        Ok(Self {
            lock_path: lock_path.to_path_buf(),
            handle,
        })
    }

    /// Update the port in the lock file (after port retry).
    pub fn update_port(&self, new_port: u16) -> Result<(), ServerLockedError> {
        let old_info = Self::read_lock_info(&self.lock_path).map_err(ServerLockedError::Io)?;
        let new_info = LockInfo {
            port: new_port,
            ..old_info
        };
        let payload = serde_json::to_string(&new_info).expect("LockInfo serialization never fails");
        // Truncate and rewrite.
        fs::write(&self.lock_path, &payload).map_err(ServerLockedError::Io)?;
        Ok(())
    }

    /// Read the lock info from an existing lock file.
    fn read_lock_info(lock_path: &Path) -> Result<LockInfo, std::io::Error> {
        let mut file = File::open(lock_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        serde_json::from_str(&contents).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("lock file parse: {e}"),
            )
        })
    }

    /// Read and return the lock info without acquiring the lock.
    pub fn probe(lock_path: &Path) -> Result<LockInfo, ServerLockedError> {
        Self::read_lock_info(lock_path).map_err(ServerLockedError::Io)
    }
}

impl Drop for ServerLock {
    fn drop(&mut self) {
        // Best-effort deletion on drop.
        let _ = fs::remove_file(&self.lock_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_lock_path() -> PathBuf {
        std::env::temp_dir().join(format!("xylitol-test-lock-{}", uuid::Uuid::new_v4()))
    }

    fn sample_info(port: u16) -> LockInfo {
        LockInfo {
            port,
            pid: std::process::id(),
            hostname: hostname(),
        }
    }

    fn hostname() -> String {
        std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("COMPUTERNAME"))
            .unwrap_or_else(|_| "unknown".into())
    }

    #[test]
    fn acquire_and_release() {
        let path = temp_lock_path();
        let info = sample_info(8080);

        // First acquire should succeed.
        let lock = ServerLock::try_acquire(&path, &info).expect("first acquire");
        assert!(path.exists(), "lock file should exist");

        // Second acquire should fail.
        let err = ServerLock::try_acquire(&path, &sample_info(8081)).unwrap_err();
        match err {
            ServerLockedError::AlreadyRunning(existing) => {
                assert_eq!(existing.port, 8080);
            }
            _ => panic!("expected AlreadyRunning error"),
        }

        // Drop lock → file should be deleted.
        drop(lock);
        assert!(!path.exists(), "lock file should be deleted on drop");

        // After drop, acquire should succeed again.
        let _relock = ServerLock::try_acquire(&path, &sample_info(8082)).expect("re-acquire");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn update_port() {
        let path = temp_lock_path();
        let lock = ServerLock::try_acquire(&path, &sample_info(8080)).expect("acquire");

        lock.update_port(8081).expect("update port");
        let info = ServerLock::probe(&path).expect("probe");
        assert_eq!(info.port, 8081);

        drop(lock);
    }

    #[test]
    fn probe_non_existent() {
        let path = temp_lock_path();
        let err = ServerLock::probe(&path).unwrap_err();
        assert!(matches!(err, ServerLockedError::Io(_)));
    }

    #[test]
    fn lock_info_serialization() {
        let info = LockInfo {
            port: 9090,
            pid: 12345,
            hostname: "test-host".into(),
        };
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: LockInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.port, 9090);
        assert_eq!(deserialized.pid, 12345);
        assert_eq!(deserialized.hostname, "test-host");
    }
}
