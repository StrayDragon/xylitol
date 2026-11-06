use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

const START_DELAY: Duration = Duration::from_millis(150);
const FRAME_INTERVAL: Duration = Duration::from_millis(80);

const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

struct ProgressState {
    writer: Mutex<Box<dyn Write + Send>>,
    stop: AtomicBool,
    emitted: AtomicBool,
}

pub struct BootProgress {
    state: Option<Arc<ProgressState>>,
    handle: Option<JoinHandle<()>>,
}

impl BootProgress {
    pub fn start_if_tty(label: &str) -> Option<Self> {
        if !std::io::stderr().is_terminal() {
            return None;
        }
        Some(Self::start_with(
            label,
            Box::new(std::io::stderr()),
            START_DELAY,
            FRAME_INTERVAL,
        ))
    }

    #[doc(hidden)]
    pub fn start_with(
        label: &str,
        writer: Box<dyn Write + Send>,
        delay: Duration,
        frame_interval: Duration,
    ) -> Self {
        let state = Arc::new(ProgressState {
            writer: Mutex::new(writer),
            stop: AtomicBool::new(false),
            emitted: AtomicBool::new(false),
        });
        let label = label.to_string();
        let worker = state.clone();
        let handle = std::thread::spawn(move || {
            std::thread::sleep(delay);
            let mut index = 0;
            loop {
                if worker.stop.load(Ordering::SeqCst) {
                    break;
                }
                let mut writer = match worker.writer.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                if worker.stop.load(Ordering::SeqCst) {
                    break;
                }
                let frame = FRAMES[index % FRAMES.len()];
                let _ = write!(writer, "\r\x1b[2K{frame} {label}");
                let _ = writer.flush();
                worker.emitted.store(true, Ordering::SeqCst);
                drop(writer);
                index += 1;
                std::thread::sleep(frame_interval);
            }
        });
        Self {
            state: Some(state),
            handle: Some(handle),
        }
    }

    pub fn finish(mut self) {
        self.stop_and_clear();
    }

    fn stop_and_clear(&mut self) {
        let Some(state) = self.state.take() else {
            return;
        };
        state.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        if state.emitted.load(Ordering::SeqCst) {
            let mut writer = match state.writer.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            let _ = write!(writer, "\r\x1b[2K");
            let _ = writer.flush();
        }
    }
}

impl Drop for BootProgress {
    fn drop(&mut self) {
        self.stop_and_clear();
    }
}

pub async fn with_boot_progress<F>(label: &str, fut: F) -> F::Output
where
    F: std::future::Future,
{
    let progress = BootProgress::start_if_tty(label);
    let output = fut.await;
    if let Some(progress) = progress {
        progress.finish();
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    #[derive(Clone)]
    struct SharedWriter(Arc<StdMutex<Vec<u8>>>);

    impl Write for SharedWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().expect("buffer").extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn buffer() -> (SharedWriter, Arc<StdMutex<Vec<u8>>>) {
        let buf = Arc::new(StdMutex::new(Vec::new()));
        (SharedWriter(buf.clone()), buf)
    }

    #[test]
    fn fast_finish_prints_nothing() {
        let (writer, buf) = buffer();
        let progress = BootProgress::start_with(
            "Starting host",
            Box::new(writer),
            Duration::from_millis(100),
            Duration::from_millis(10),
        );
        progress.finish();
        assert!(buf.lock().expect("buffer").is_empty());
    }

    #[test]
    fn slow_wait_prints_frames_then_clears() {
        let (writer, buf) = buffer();
        let progress = BootProgress::start_with(
            "Starting host",
            Box::new(writer),
            Duration::from_millis(10),
            Duration::from_millis(10),
        );
        std::thread::sleep(Duration::from_millis(80));
        progress.finish();
        let text = String::from_utf8(buf.lock().expect("buffer").clone()).expect("utf8");
        assert!(text.contains("Starting host"), "{text}");
        assert!(text.ends_with("\r\x1b[2K"), "{text:?}");
    }

    #[test]
    fn writer_failure_is_silent() {
        struct Failing;

        impl Write for Failing {
            fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("closed"))
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::Error::other("closed"))
            }
        }

        let progress = BootProgress::start_with(
            "Starting host",
            Box::new(Failing),
            Duration::from_millis(5),
            Duration::from_millis(5),
        );
        std::thread::sleep(Duration::from_millis(30));
        progress.finish();
    }

    #[test]
    fn non_tty_is_disabled() {
        if !std::io::stderr().is_terminal() {
            assert!(BootProgress::start_if_tty("Starting host").is_none());
        }
    }

    #[tokio::test]
    async fn with_boot_progress_propagates_output() {
        let value = with_boot_progress("Starting host", async { 42 }).await;
        assert_eq!(value, 42);
    }
}
