//! Standard filesystem implementation of the [`XyExportIo`] port.

use std::path::Path;

use async_trait::async_trait;

use crate::runtime_protocol::XyExportIo;

/// Filesystem-backed [`XyExportIo`] using `tokio::fs`.
#[derive(Debug, Default, Clone)]
pub struct StdExportIo;

impl StdExportIo {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl XyExportIo for StdExportIo {
    async fn write_text(&self, path: &Path, content: &str) -> Result<(), String> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("create dirs: {e}"))?;
        }
        tokio::fs::write(path, content)
            .await
            .map_err(|e| format!("write {}: {e}", path.display()))
    }

    async fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String> {
        tokio::fs::read(path)
            .await
            .map_err(|e| format!("read {}: {e}", path.display()))
    }
}
