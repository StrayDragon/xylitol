#![allow(dead_code)] // WIP: not yet integrated into main flow

/// Session system configuration.
#[derive(Debug, Clone)]
pub(crate) struct SessionConfig {
    pub(crate) auto_snapshot: bool,
    pub(crate) max_snapshots: usize,
    pub(crate) storage: StorageConfig,
    pub(crate) compaction: CompactionConfig,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            auto_snapshot: true,
            max_snapshots: 50,
            storage: StorageConfig::default(),
            compaction: CompactionConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StorageConfig {
    pub(crate) backend: StorageBackend,
    pub(crate) compress: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: StorageBackend::Sqlite,
            compress: true,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum StorageBackend {
    Sqlite,
}

#[derive(Debug, Clone)]
pub(crate) struct CompactionConfig {
    pub(crate) strategy: CompactionStrategy,
    /// Fraction of context window that triggers auto-compaction (0.0–1.0).
    pub(crate) token_threshold: f64,
    /// Keep tool-call summaries in compacted output.
    pub(crate) keep_tool_summaries: bool,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            strategy: CompactionStrategy::Intra,
            token_threshold: 0.75,
            keep_tool_summaries: true,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum CompactionStrategy {
    Intra,
    Manual,
    Derive,
}
