//! File system watching utility.
//!
//! Wraps the `notify` crate for file change monitoring.

use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

/// Watch a file for changes, calling `callback` when modified.
///
/// Returns a channel sender; drop it to stop watching.
pub fn watch_file(path: &Path, callback: Box<dyn Fn() + Send + 'static>) -> mpsc::Sender<()> {
    let (tx, rx) = mpsc::channel::<()>();
    let path = path.to_path_buf();

    std::thread::spawn(move || {
        use notify::{Config, Event, EventKind, RecommendedWatcher, Watcher};

        let (event_tx, event_rx) = mpsc::channel::<Result<Event, notify::Error>>();

        let mut watcher = match RecommendedWatcher::new(
            move |res| {
                let _ = event_tx.send(res);
            },
            Config::default(),
        ) {
            Ok(w) => w,
            Err(e) => {
                log::warn!("fs_watch: failed to create watcher: {e}");
                return;
            }
        };

        if let Err(e) = watcher.watch(&path, notify::RecursiveMode::NonRecursive) {
            log::warn!("fs_watch: failed to watch {path:?}: {e}");
            return;
        }

        loop {
            // Check for stop signal
            if rx.try_recv().is_ok() {
                break;
            }

            // Check for file events with timeout
            match event_rx.recv_timeout(Duration::from_millis(500)) {
                Ok(Ok(event)) => {
                    if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                        // Debounce: small delay to avoid double triggers
                        std::thread::sleep(Duration::from_millis(100));
                        callback();
                    }
                }
                Ok(Err(e)) => {
                    log::warn!("fs_watch: watch error: {e}");
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Normal timeout — just loop
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    tx
}
