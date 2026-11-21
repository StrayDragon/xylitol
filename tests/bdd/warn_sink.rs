use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

static RECORDS: Mutex<Vec<(log::Level, String, String)>> = Mutex::new(Vec::new());

static INSTALLED: AtomicBool = AtomicBool::new(false);

struct Sink;

impl log::Log for Sink {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &log::Record<'_>) {
        RECORDS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push((
                record.level(),
                record.target().to_string(),
                record.args().to_string(),
            ));
    }

    fn flush(&self) {}
}

static SINK: Sink = Sink;

pub(crate) fn install() {
    if INSTALLED.swap(true, Ordering::SeqCst) {
        log::set_max_level(log::LevelFilter::Warn);
        return;
    }
    let _ = log::set_logger(&SINK);
    log::set_max_level(log::LevelFilter::Warn);
}

pub(crate) fn reset() {
    install();
    RECORDS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clear();
}

pub(crate) fn take_warnings(target: &str) -> Vec<String> {
    RECORDS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .drain(..)
        .filter(|(level, record_target, _)| *level == log::Level::Warn && record_target == target)
        .map(|(_, _, message)| message)
        .collect()
}
