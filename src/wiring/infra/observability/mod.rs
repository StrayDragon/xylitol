//! Observability composition helpers (fastrace FileReporter + optional OTLP).

pub mod fanout;
pub mod file_reporter;
pub mod otel;

pub use fanout::FanoutReporter;
pub use file_reporter::FileTraceReporter;
