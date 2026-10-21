use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;

#[derive(Clone)]
pub struct FileMutationQueue {
    queues: Arc<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>>,
}

impl FileMutationQueue {
    pub fn new() -> Self {
        Self {
            queues: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn run<F, Fut, T, E>(&self, file_path: &str, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<T, E>> + Send,
        T: Send,
    {
        let key = match tokio::fs::canonicalize(file_path).await {
            Ok(real) => real,
            Err(_) => PathBuf::from(file_path),
        };

        let path_lock = {
            let mut map = self.queues.lock().await;
            map.entry(key.clone())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };

        let _guard = path_lock.lock().await;

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

    const OVERLAP_DELAY: Duration = Duration::from_millis(50);

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
                tokio::time::sleep(OVERLAP_DELAY).await;
                Ok::<_, ()>(c1.fetch_add(1, Ordering::SeqCst))
            }),
            q2.run("b.txt", || async move {
                tokio::time::sleep(OVERLAP_DELAY).await;
                Ok::<_, ()>(c2.fetch_add(1, Ordering::SeqCst))
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
                tokio::time::sleep(OVERLAP_DELAY).await;
                c.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ()>(())
            }),
            q2.run("shared.txt", || async {
                c.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ()>(())
            }),
        );

        assert!(r1.is_ok());
        assert!(r2.is_ok());
        assert_eq!(counter.load(Ordering::SeqCst), 2);

        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(40),
            "serialized ops should wait: {elapsed:?}"
        );
    }
}
