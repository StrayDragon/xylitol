use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceScope {
    User,
    Project,
    Temporary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceOrigin {
    Package,
    TopLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    pub path: PathBuf,
    pub source: String,
    pub scope: SourceScope,
    pub origin: SourceOrigin,
    pub base_dir: Option<PathBuf>,
}

pub fn create_source_info(
    path: PathBuf,
    source: String,
    scope: SourceScope,
    origin: SourceOrigin,
    base_dir: Option<PathBuf>,
) -> SourceInfo {
    SourceInfo {
        path,
        source,
        scope,
        origin,
        base_dir,
    }
}
