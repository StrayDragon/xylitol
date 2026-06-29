//! Runtime boundary for session export/import I/O.

use std::path::Path;

use async_trait::async_trait;

/// Port for writing and reading export files.
///
/// Async from day one so backends like gist, S3, or clipboard can be
/// plugged in without touching the agent layer.
#[async_trait]
pub trait ExportIo: Send + Sync {
    /// Write `content` as UTF-8 text to `path`.
    async fn write_text(&self, path: &Path, content: &str) -> Result<(), String>;

    /// Read `path` as raw bytes.
    async fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String>;
}
