use crate::error::XyExportError;
use std::path::Path;

use async_trait::async_trait;

#[async_trait]
pub trait XyExportIo: Send + Sync {
    async fn write_text(&self, path: &Path, content: &str) -> Result<(), XyExportError>;

    async fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, XyExportError>;
}
