use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub const DEFAULT_STALE_AFTER: Duration = Duration::from_secs(30);
const MAX_ATTEMPTS: u32 = 10;
const BACKOFF_BASE: Duration = Duration::from_millis(20);

#[derive(Debug)]
pub struct FileLockGuard {
    path: PathBuf,
}

impl FileLockGuard {
    pub fn acquire(path: impl Into<PathBuf>) -> std::io::Result<Self> {
        Self::acquire_with_stale(path.into(), DEFAULT_STALE_AFTER)
    }

    pub fn acquire_with_stale(path: PathBuf, stale_after: Duration) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut delay = BACKOFF_BASE;
        for attempt in 1..=MAX_ATTEMPTS {
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&path) {
                Ok(mut file) => {
                    let _ = writeln!(file, "{}", std::process::id());
                    return Ok(Self { path });
                }
                Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                    if is_stale(&path, stale_after) {
                        let _ = fs::remove_file(&path);
                        continue;
                    }
                    if attempt == MAX_ATTEMPTS {
                        break;
                    }
                    std::thread::sleep(delay);
                    delay = (delay.mul_f64(1.5)).min(Duration::from_secs(1));
                }
                Err(e) => return Err(e),
            }
        }
        Err(std::io::Error::new(
            ErrorKind::AlreadyExists,
            "file lock acquire failed after retries",
        ))
    }

    pub fn release(self) {
        drop(self);
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for FileLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn is_stale(path: &Path, stale_after: Duration) -> bool {
    fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .is_some_and(|modified| {
            SystemTime::now()
                .duration_since(modified)
                .is_ok_and(|age| age >= stale_after)
        })
}
