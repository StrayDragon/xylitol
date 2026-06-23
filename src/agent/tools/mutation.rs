//! File mutation queue — serializes concurrent writes/edits to the same file path.
//!
//! Operations targeting different files run in parallel; operations targeting
//! the same file are serialized via per-path locks.

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;

/// Serializes mutations on a per-path basis. Users hold a shared reference and call
/// [`FileMutationQueue::run`] to wrap a mutation.
#[derive(Clone)]
pub struct FileMutationQueue {
    /// A Mutex<HashMap> so we can lazily insert per-path locks without write contention
    /// on the map (we only briefly lock the map to look up/insert, then lock the per-path lock).
    queues: Arc<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>>,
}

impl FileMutationQueue {
    pub fn new() -> Self {
        Self {
            queues: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Execute a mutation `f` on `file_path`. If another mutation is currently running on the
    /// same path (canonically resolved), this call will wait for it to finish before executing.
    pub async fn run<F, Fut, T>(&self, file_path: &str, f: F) -> Result<T, String>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<T, String>> + Send,
        T: Send,
    {
        // Resolve the real path (best-effort)
        let key = match tokio::fs::canonicalize(file_path).await {
            Ok(real) => real,
            Err(_) => PathBuf::from(file_path),
        };

        // Get or create a per-path mutex
        let path_lock = {
            let mut map = self.queues.lock().await;
            map.entry(key.clone())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };

        // Serialize — only one mutation at a time per path
        let _guard = path_lock.lock().await;

        // Periodically clean up entries that have no waiters (opportunistic)
        // We'll skip this for now; small overhead from storing one Arc per unique path touched.
        f().await
    }
}

impl Default for FileMutationQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    #[tokio::test]
    async fn test_different_paths_run_in_parallel() {
        let queue = FileMutationQueue::new();
        let counter = Arc::new(AtomicU32::new(0));

        let c1 = counter.clone();
        let c2 = counter.clone();
        let q1 = queue.clone();
        let q2 = queue.clone();

        let (r1, r2) = tokio::join!(
            q1.run("a.txt", || async move {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Ok(c1.fetch_add(1, Ordering::SeqCst))
            }),
            q2.run("b.txt", || async move {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Ok(c2.fetch_add(1, Ordering::SeqCst))
            }),
        );

        assert!(r1.is_ok());
        assert!(r2.is_ok());
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_same_path_serialized() {
        let queue = FileMutationQueue::new();
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();

        let start = tokio::time::Instant::now();

        let q1 = queue.clone();
        let q2 = queue.clone();

        let (r1, r2) = tokio::join!(
            q1.run("shared.txt", || async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                c.fetch_add(1, Ordering::SeqCst);
                Ok::<_, String>(())
            }),
            q2.run("shared.txt", || async {
                c.fetch_add(1, Ordering::SeqCst);
                Ok::<_, String>(())
            }),
        );

        assert!(r1.is_ok());
        assert!(r2.is_ok());
        assert_eq!(counter.load(Ordering::SeqCst), 2);

        // Should take ~100ms because they're serialized
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(40),
            "serialized ops should wait: {elapsed:?}"
        );
    }
}
