//! Cross-process write occupancy for one session directory (r1950).
//!
//! Aligns with deepseek-harness: the arbiter is the kernel. POSIX `flock`s an
//! unchanging `session.lock` (never rewrite, never unlink). Windows holds a
//! named semaphore derived from that path and never file-locks session data.
//! Same-process dual clients are serialized by Host `run_inflight`, not here.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use super::SessionManager;
use crate::infra::session::types::SessionBackend;
use crate::protocol::error::XySessionStoreError;
use crate::utils::lock_mutex;

/// Base name of the kernel lock file inside a session directory (dsh).
pub const SESSION_WRITE_LOCK_FILENAME: &str = "session.lock";

/// Guard whose Drop releases the kernel exclusive occupancy.
pub struct SessionWriteLock {
    /// Held so Drop closes the POSIX fd or Windows semaphore handle.
    #[allow(dead_code)]
    inner: Inner,
}

impl std::fmt::Debug for SessionWriteLock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SessionWriteLock")
    }
}

enum Inner {
    #[cfg(unix)]
    Posix(#[allow(dead_code)] File),
    #[cfg(windows)]
    Win32(win32::Semaphore),
}

impl SessionWriteLock {
    /// Take occupancy for a materialized session directory.
    pub fn try_acquire(dir: &Path, session_id: &str) -> Result<Self, XySessionStoreError> {
        let path = dir.join(SESSION_WRITE_LOCK_FILENAME);
        #[cfg(unix)]
        {
            posix_acquire(&path, session_id)
        }
        #[cfg(windows)]
        {
            win32::acquire(&path, session_id)
        }
    }
}

impl SessionManager {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn write_lock_path(&self, session_id: &str) -> PathBuf {
        self.session_dir_path(session_id)
            .join(SESSION_WRITE_LOCK_FILENAME)
    }

    /// Take (or keep) process-local occupancy for a materialized session.
    ///
    /// No-op for in-memory stores and sessions with no manifest yet.
    pub fn acquire_write_lock(&self, session_id: &str) -> Result<(), XySessionStoreError> {
        if matches!(self.backend, SessionBackend::InMemory) {
            return Ok(());
        }
        if !self.manifest_path(session_id).exists() {
            return Ok(());
        }
        let mut map = lock_mutex(&self.write_locks);
        if map.contains_key(session_id) {
            return Ok(());
        }
        let lock = SessionWriteLock::try_acquire(&self.session_dir_path(session_id), session_id)?;
        map.insert(session_id.to_string(), lock);
        Ok(())
    }

    /// Close the occupancy. Kernel releases; `session.lock` stays on disk.
    pub fn release_write_lock(&self, session_id: &str) {
        lock_mutex(&self.write_locks).remove(session_id);
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn holds_write_lock(&self, session_id: &str) -> bool {
        lock_mutex(&self.write_locks).contains_key(session_id)
    }
}

#[cfg(unix)]
fn posix_acquire(path: &Path, session_id: &str) -> Result<SessionWriteLock, XySessionStoreError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| XySessionStoreError::io("create session dir for write lock", e))?;
    }
    // Bounded retry: locking an inode a releasing creator unlinked (or a
    // recreated path) re-opens the fresh file; dsh does three passes.
    for _ in 0..3 {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|e| XySessionStoreError::io("open session.lock", e))?;
        flock_exclusive_nb(&file, session_id)?;
        if inode_still_at_path(&file, path)? {
            return Ok(SessionWriteLock {
                inner: Inner::Posix(file),
            });
        }
        drop(file);
    }
    Err(XySessionStoreError::write_held(session_id))
}

#[cfg(unix)]
fn flock_exclusive_nb(file: &File, session_id: &str) -> Result<(), XySessionStoreError> {
    use std::os::unix::io::AsRawFd;
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc == 0 {
        return Ok(());
    }
    let err = std::io::Error::last_os_error();
    if err.kind() == std::io::ErrorKind::WouldBlock || err.raw_os_error() == Some(libc::EWOULDBLOCK)
    {
        return Err(XySessionStoreError::write_held(session_id));
    }
    Err(XySessionStoreError::io("flock session.lock", err))
}

#[cfg(unix)]
fn inode_still_at_path(file: &File, path: &Path) -> Result<bool, XySessionStoreError> {
    use std::os::unix::fs::MetadataExt;
    let held = file
        .metadata()
        .map_err(|e| XySessionStoreError::io("stat locked session.lock", e))?;
    let current = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(XySessionStoreError::io("stat session.lock path", e)),
    };
    Ok(held.dev() == current.dev() && held.ino() == current.ino())
}

#[cfg(windows)]
mod win32 {
    use std::path::Path;

    use sha2::{Digest, Sha256};

    use super::{Inner, SessionWriteLock};
    use crate::protocol::error::XySessionStoreError;

    const WAIT_OBJECT_0: u32 = 0;
    const WAIT_TIMEOUT: u32 = 0x0000_0102;

    pub struct Semaphore(isize);

    impl Drop for Semaphore {
        fn drop(&mut self) {
            unsafe {
                let _ = ReleaseSemaphore(self.0, 1, std::ptr::null_mut());
                let _ = CloseHandle(self.0);
            }
        }
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateSemaphoreW(
            security: *const core::ffi::c_void,
            initial_count: i32,
            maximum_count: i32,
            name: *const u16,
        ) -> isize;
        fn WaitForSingleObject(handle: isize, milliseconds: u32) -> u32;
        fn ReleaseSemaphore(handle: isize, release_count: i32, previous: *mut i32) -> i32;
        fn CloseHandle(handle: isize) -> i32;
        fn GetLastError() -> u32;
    }

    pub fn acquire(path: &Path, session_id: &str) -> Result<SessionWriteLock, XySessionStoreError> {
        let name = semaphore_name(path);
        let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let handle = unsafe { CreateSemaphoreW(std::ptr::null(), 1, 1, wide.as_ptr()) };
        if handle == 0 {
            let code = unsafe { GetLastError() };
            return Err(XySessionStoreError::io(
                "CreateSemaphoreW session lock",
                std::io::Error::from_raw_os_error(code as i32),
            ));
        }
        let wait = unsafe { WaitForSingleObject(handle, 0) };
        if wait == WAIT_OBJECT_0 {
            return Ok(SessionWriteLock {
                inner: Inner::Win32(Semaphore(handle)),
            });
        }
        unsafe {
            let _ = CloseHandle(handle);
        }
        if wait == WAIT_TIMEOUT {
            return Err(XySessionStoreError::write_held(session_id));
        }
        Err(XySessionStoreError::io(
            "WaitForSingleObject session lock",
            std::io::Error::from_raw_os_error(unsafe { GetLastError() } as i32),
        ))
    }

    fn semaphore_name(path: &Path) -> String {
        let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let key = canon.to_string_lossy().to_lowercase();
        let digest = Sha256::digest(key.as_bytes());
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        format!("Local\\xylitol-session-lock-{hex}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::session::manager::SessionManager;
    use crate::protocol::session::{EntryBase, MessageEntry, SessionEntry};

    fn assistant(text: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            message: crate::protocol::session::fixture_message_json("assistant", text),
        })
    }

    async fn materialize(mgr: &SessionManager, id: &str) {
        mgr.create(id, Some("."), None).await.expect("create");
        mgr.append(id, &assistant("hi")).await.expect("flush");
        assert!(mgr.manifest_path(id).exists(), "manifest must exist");
    }

    #[tokio::test]
    async fn same_process_reacquire_via_manager_is_ok() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SessionManager::new(dir.path().join("sessions"));
        materialize(&mgr, "s").await;
        mgr.acquire_write_lock("s").expect("first");
        assert!(mgr.holds_write_lock("s"));
        mgr.acquire_write_lock("s").expect("same process continues");
        assert!(mgr.write_lock_path("s").exists(), "session.lock stays");
    }

    #[cfg(unix)]
    #[test]
    fn other_process_flock_conflicts_then_release_allows() {
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join(SESSION_WRITE_LOCK_FILENAME);
        let held = SessionWriteLock::try_acquire(dir.path(), "s").expect("hold");
        assert!(lock_path.exists());

        let script = format!(
            "import fcntl, sys\nf=open({path:?}, 'r+')\ntry:\n fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)\n sys.exit(0)\nexcept BlockingIOError:\n sys.exit(2)\n",
            path = lock_path
        );
        let status = std::process::Command::new("python3")
            .arg("-c")
            .arg(&script)
            .status()
            .expect("python3 flock probe");
        assert_eq!(
            status.code(),
            Some(2),
            "other process MUST fail while we hold {status:?}"
        );

        drop(held);
        let status = std::process::Command::new("python3")
            .arg("-c")
            .arg(&script)
            .status()
            .expect("python3 flock probe after drop");
        assert_eq!(
            status.code(),
            Some(0),
            "other process MUST succeed after Drop {status:?}"
        );
        assert!(lock_path.exists(), "release MUST NOT unlink session.lock");
    }

    #[tokio::test]
    async fn second_manager_conflicts_on_materialized_session() {
        let dir = tempfile::tempdir().unwrap();
        let sessions = dir.path().join("sessions");
        let a = SessionManager::new(sessions.clone());
        materialize(&a, "s1").await;
        a.acquire_write_lock("s1").expect("A holds");

        #[cfg(unix)]
        {
            let script = format!(
                "import fcntl, sys\nf=open({path:?}, 'r+')\ntry:\n fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)\n sys.exit(0)\nexcept BlockingIOError:\n sys.exit(2)\n",
                path = a.write_lock_path("s1")
            );
            let status = std::process::Command::new("python3")
                .arg("-c")
                .arg(&script)
                .status()
                .expect("python3");
            assert_eq!(status.code(), Some(2), "B MUST not take A's lock");
        }

        a.release_write_lock("s1");
        let b = SessionManager::new(sessions);
        b.acquire_write_lock("s1")
            .expect("B takes after A released");
        b.append("s1", &assistant("from-b"))
            .await
            .expect("B can append");
    }

    #[tokio::test]
    async fn in_memory_does_not_require_lock() {
        let mgr = SessionManager::in_memory();
        mgr.create("mem", Some("."), None).await.unwrap();
        mgr.acquire_write_lock("mem").expect("in-memory is a no-op");
        mgr.append("mem", &assistant("x")).await.unwrap();
    }

    #[tokio::test]
    async fn does_not_lock_manifest_inode() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SessionManager::new(dir.path().join("sessions"));
        materialize(&mgr, "s1").await;
        mgr.acquire_write_lock("s1").expect("hold");
        let manifest = mgr.manifest_path("s1");
        let lock = mgr.write_lock_path("s1");
        assert_ne!(manifest, lock);
        assert!(lock.ends_with(SESSION_WRITE_LOCK_FILENAME));
    }
}
