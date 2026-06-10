//! Pluggable operations traits for tool execution.
//!
//! Each tool delegates its actual I/O through a trait, allowing:
//! - Default implementations using tokio::fs / tokio::process
//! - Test implementations using in-memory backends
//! - Hook interception at the operations boundary

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;

// ── Bash ────────────────────────────────────────────────────────────

pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[async_trait]
pub trait BashOperations: Send + Sync {
    /// Execute a shell command and return output.
    async fn execute_command(&self, cmd: &str, timeout: Duration) -> Result<CommandOutput, String>;

    /// Send a kill signal to a process tree.
    async fn kill_tree(&self, pid: u32) -> Result<(), String>;
}

// ── Read ────────────────────────────────────────────────────────────

pub struct FileMetadata {
    pub size: u64,
    pub is_dir: bool,
    pub is_file: bool,
}

#[async_trait]
pub trait ReadOperations: Send + Sync {
    /// Read the full content of a file.
    async fn read_file(&self, path: &str) -> Result<String, String>;

    /// Get file metadata.
    async fn metadata(&self, path: &str) -> Result<FileMetadata, String>;
}

// ── Write / Edit ────────────────────────────────────────────────────

#[async_trait]
pub trait WriteOperations: Send + Sync {
    /// Create parent directories for a file path.
    async fn create_parent_dirs(&self, path: &str) -> Result<(), String>;

    /// Write content to a file atomically (temp + rename).
    async fn write_file(&self, path: &str, content: &str) -> Result<(), String>;

    /// Read existing content of a file.
    async fn read_file(&self, path: &str) -> Result<String, String>;
}

// ── Edit-specific operations are covered by WriteOperations + BOM/unicode handling ──

// ── Grep ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GrepMatch {
    pub file: PathBuf,
    pub line_number: u64,
    pub content: String,
    /// Matching substring(s) with optional byte range
    pub match_ranges: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, Default)]
pub struct GrepOptions {
    pub regex: bool,
    pub glob: Option<String>,
    pub ignore_case: bool,
    pub literal: bool,
    pub context_lines: Option<u32>,
    pub limit: Option<u32>,
}

#[async_trait]
pub trait GrepOperations: Send + Sync {
    /// Search for pattern in path using ripgrep.
    async fn grep(
        &self,
        pattern: &str,
        path: &str,
        options: GrepOptions,
    ) -> Result<Vec<GrepMatch>, String>;
}

// ── Find ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FindResult {
    pub path: PathBuf,
    pub is_dir: bool,
}

#[async_trait]
pub trait FindOperations: Send + Sync {
    /// Find files matching glob pattern under root path, using fd.
    async fn find(
        &self,
        pattern: &str,
        root_path: &str,
        limit: Option<u32>,
    ) -> Result<Vec<FindResult>, String>;
}

// ── Ls ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LsEntry {
    pub name: String,
    pub is_dir: bool,
    pub is_symlink: bool,
}

#[async_trait]
pub trait LsOperations: Send + Sync {
    /// List entries in a directory.
    async fn list_dir(&self, path: &str) -> Result<Vec<LsEntry>, String>;
}
