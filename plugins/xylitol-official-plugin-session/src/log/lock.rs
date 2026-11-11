use std::fs::{File, OpenOptions};
use std::path::Path;

use xylitol_plugin_sdk::error::XySessionStoreError;

pub const SESSION_WRITE_LOCK_FILENAME: &str = "session.lock";

pub(crate) struct SessionWriteLock {
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
    pub(crate) fn try_acquire(dir: &Path, session_id: &str) -> Result<Self, XySessionStoreError> {
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

#[cfg(unix)]
fn posix_acquire(path: &Path, session_id: &str) -> Result<SessionWriteLock, XySessionStoreError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| XySessionStoreError::io("create session dir for write lock", e))?;
    }
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
    use std::hash::{Hash, Hasher};
    use std::path::Path;

    use super::{Inner, SessionWriteLock};
    use xylitol_plugin_sdk::error::XySessionStoreError;

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
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        format!("Local\\xylitol-session-lock-{:016x}", hasher.finish())
    }
}
