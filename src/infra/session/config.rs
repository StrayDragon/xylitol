#![allow(dead_code)] // WIP: not yet integrated into main flow

/// Session system configuration.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub auto_snapshot: bool,
    pub max_snapshots: usize,
    pub storage: StorageConfig,
    pub compaction: CompactionConfig,
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
pub struct StorageConfig {
    pub backend: StorageBackend,
    pub compress: bool,
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
pub enum StorageBackend {
    Sqlite,
}

#[derive(Debug, Clone)]
pub struct CompactionConfig {
    pub strategy: CompactionStrategy,
    /// Fraction of context window that triggers auto-compaction (0.0–1.0).
    pub token_threshold: f64,
    /// Keep tool-call summaries in compacted output.
    pub keep_tool_summaries: bool,
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
pub enum CompactionStrategy {
    Intra,
    Manual,
    Derive,
}
