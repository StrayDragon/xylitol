pub(crate) mod compaction;
pub(crate) mod config;
pub(crate) mod fine_tune;
pub(crate) mod gc;
pub(crate) mod manager;
pub(crate) mod storage;
pub(crate) mod types;

// Re-exports consumed by upper layers (integrated in agent setup).
#[allow(unused_imports)]
pub(crate) use config::SessionConfig;
#[allow(unused_imports)]
pub(crate) use manager::SnapshotManager;

#[cfg(test)]
pub(crate) mod tests;
