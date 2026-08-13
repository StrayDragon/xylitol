//! OutputAccumulator — streaming output buffer with temp file spillover.
//!
//! Provides:
//! - Incremental data append (raw bytes)
//! - Rolling memory buffer (configurable max bytes)
//! - Transparent temp file spillover when buffer exceeds threshold
//! - Final snapshot with truncation info

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use super::truncate::DEFAULT_MAX_BYTES;

/// Default rolling buffer size (2x max_bytes before spilling to temp).
const DEFAULT_MAX_ROLLING_BYTES_FACTOR: usize = 2;

/// Rolling buffer that accumulates output data.
///
/// - Data is first collected in memory (pre_chunks).
/// - When total bytes exceeds `max_rolling_bytes`, a temp file is created
///   and all accumulated data is written to it.
/// - Further appends go directly to the temp file.
/// - A rolling text buffer keeps the tail in memory for quick access.
pub(crate) struct OutputAccumulator {
    /// Max bytes to keep in the rolling text buffer (tail).
    max_bytes: usize,
    /// Max bytes before spilling to temp file.
    max_rolling_bytes: usize,
    /// Rolling text buffer (tail of the output).
    rolling_text: String,
    /// Current bytes in rolling_text.
    rolling_bytes: usize,
    /// Temp file path (created when overflow occurs).
    temp_file: Option<PathBuf>,
    /// Temp file writer.
    temp_writer: Option<BufWriter<File>>,
    /// Total bytes received.
    total_bytes: usize,
    /// Pre-temp-file data chunks (kept until temp file is created, then written).
    pre_chunks: Vec<Vec<u8>>,
    /// Whether a temp file has been opened.
    spilled_to_file: bool,
}

impl Default for OutputAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputAccumulator {
    /// Create a new OutputAccumulator with default limits.
    pub fn new() -> Self {
        Self::with_limits(
            DEFAULT_MAX_BYTES,
            DEFAULT_MAX_BYTES * DEFAULT_MAX_ROLLING_BYTES_FACTOR,
        )
    }

    /// Create with custom limits.
    pub fn with_limits(max_bytes: usize, max_rolling_bytes: usize) -> Self {
        Self {
            max_bytes,
            max_rolling_bytes,
            rolling_text: String::new(),
            rolling_bytes: 0,
            temp_file: None,
            temp_writer: None,
            total_bytes: 0,
            pre_chunks: Vec::new(),
            spilled_to_file: false,
        }
    }

    /// Append raw data to the accumulator.
    pub fn append(&mut self, data: &[u8]) {
        self.total_bytes += data.len();

        if self.spilled_to_file {
            // Write directly to temp file
            if let Some(ref mut writer) = self.temp_writer {
                let _ = writer.write_all(data);
            }
            // Also update rolling tail buffer
            self.append_rolling(data);
        } else {
            // Keep in memory
            let current_total =
                self.pre_chunks.iter().map(|c| c.len()).sum::<usize>() + self.pre_chunks.len(); // newlines

            if current_total + data.len() > self.max_rolling_bytes {
                // Exceeded rolling limit — create temp file and spill
                self.spill_to_file();
                if let Some(ref mut writer) = self.temp_writer {
                    let _ = writer.write_all(data);
                }
                self.append_rolling(data);
            } else {
                self.pre_chunks.push(data.to_vec());
                self.append_rolling(data);
            }
        }
    }

    /// Append data to the rolling text buffer (tail).
    fn append_rolling(&mut self, data: &[u8]) {
        let text = String::from_utf8_lossy(data);
        self.rolling_bytes += text.len();
        self.rolling_text.push_str(&text);

        // Trim from head if rolling buffer exceeds max_bytes
        if self.rolling_bytes > self.max_bytes {
            let excess = self.rolling_bytes - self.max_bytes;
            // Find the nearest UTF-8 character boundary at or after `excess` bytes
            let trim_at = self
                .rolling_text
                .char_indices()
                .find(|(byte_pos, _)| *byte_pos >= excess)
                .map(|(i, _)| i)
                .unwrap_or(self.rolling_text.len());
            self.rolling_text = self.rolling_text[trim_at..].to_string();
            self.rolling_bytes = self.rolling_text.len();
        }
    }

    /// Spill all in-memory data to a temp file.
    fn spill_to_file(&mut self) {
        if self.spilled_to_file {
            return;
        }

        // Create temp file
        let temp_path =
            std::env::temp_dir().join(format!("xylitol-output-{}.txt", uuid::Uuid::new_v4()));

        match File::create(&temp_path) {
            Ok(file) => {
                let mut writer = BufWriter::new(file);

                // Write all pre-chunks
                for chunk in &self.pre_chunks {
                    let _ = writer.write_all(chunk);
                }

                // Flush
                let _ = writer.flush();

                self.temp_file = Some(temp_path);
                self.temp_writer = Some(writer);
                self.spilled_to_file = true;
                self.pre_chunks.clear();
            }
            Err(_) => {
                // Can't create temp file — keep accumulating in memory
            }
        }
    }

    /// Finish accumulation and return a snapshot.
    ///
    /// Closes the temp file if one was created.
    pub fn finish(&mut self) -> OutputSnapshot {
        // Flush and close temp file
        if let Some(mut writer) = self.temp_writer.take() {
            let _ = writer.flush();
        }

        // Build full content from pre_chunks or temp file
        let full_content = if self.spilled_to_file {
            self.temp_file
                .as_ref()
                .and_then(|p| std::fs::read_to_string(p).ok())
                .unwrap_or_default()
        } else {
            self.pre_chunks
                .iter()
                .map(|c| String::from_utf8_lossy(c).to_string())
                .collect::<Vec<_>>()
                .concat()
        };

        let truncated = self.total_bytes > self.max_bytes;

        OutputSnapshot {
            content: self.rolling_text.clone(),
            full_content,
            total_bytes: self.total_bytes,
            truncated,
            max_bytes: self.max_bytes,
            full_output_path: self.temp_file.clone(),
        }
    }
}

/// Result of output accumulation.
///
/// Provides the tail of the output (for display), truncation information,
/// and a path to the full output if spilled to file.
#[derive(Debug, Clone)]
pub(crate) struct OutputSnapshot {
    /// Tail of the output (for display).
    pub(crate) content: String,
    /// Full output content.
    #[allow(dead_code)]
    pub(crate) full_content: String,
    /// Total bytes accumulated.
    #[allow(dead_code)]
    pub(crate) total_bytes: usize,
    /// Whether the output was truncated.
    pub(crate) truncated: bool,
    /// Max rolling tail bytes applied (for footer limit text).
    pub(crate) max_bytes: usize,
    /// Path to full output if spilled to temp file.
    pub(crate) full_output_path: Option<PathBuf>,
}

impl OutputSnapshot {
    /// Content for display / LLM context, with pi-shaped Full output footer when truncated.
    pub fn display_content(&self) -> String {
        if !self.truncated {
            return self.content.clone();
        }
        let path = self
            .full_output_path
            .as_ref()
            .and_then(|p| p.to_str())
            .unwrap_or("(unavailable)");
        let lines_shown = if self.content.is_empty() {
            0
        } else {
            self.content.lines().count()
        };
        let body = self.content.trim_end_matches('\n');
        format!(
            "{body}\n[Full output: {path}. Truncated: {lines_shown} lines shown ({} limit)]",
            format_size(self.max_bytes),
        )
    }
}

/// Format bytes as human-readable size.
fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes}B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_output_no_truncation() {
        let mut acc = OutputAccumulator::with_limits(1024, 2048);
        acc.append(b"hello world");
        let snapshot = acc.finish();
        assert_eq!(snapshot.content, "hello world");
        assert!(!snapshot.truncated);
        assert_eq!(snapshot.total_bytes, 11);
    }

    #[test]
    fn test_overflow_creates_temp_file() {
        let mut acc = OutputAccumulator::with_limits(100, 200);
        let data = vec![b'a'; 300];
        acc.append(&data);
        let snapshot = acc.finish();
        assert!(snapshot.truncated);
        assert!(snapshot.full_output_path.is_some());
        assert_eq!(snapshot.total_bytes, 300);
    }

    #[test]
    fn test_rolling_buffer_tail() {
        let mut acc = OutputAccumulator::with_limits(50, 100);
        // Write 90 bytes — should fit in pre_chunks, tail keeps last 50
        acc.append(b"123456789012345678901234567890");
        acc.append(b"123456789012345678901234567890");
        acc.append(b"123456789012345678901234567890");
        let snapshot = acc.finish();

        // rolling_text should be at most 50 bytes
        assert!(snapshot.content.len() <= 50);
        // full_content should have all 90 bytes
        assert_eq!(snapshot.full_content.len(), 90);
    }

    #[test]
    fn test_multiple_appends() {
        let mut acc = OutputAccumulator::with_limits(1024, 2048);
        acc.append(b"line1\n");
        acc.append(b"line2\n");
        acc.append(b"line3\n");
        let snapshot = acc.finish();
        assert_eq!(snapshot.full_content, "line1\nline2\nline3\n");
    }

    #[test]
    fn test_display_content_with_truncation() {
        let mut acc = OutputAccumulator::with_limits(10, 20);
        acc.append(b"this is a very long output that exceeds the limit");
        let snapshot = acc.finish();
        let display = snapshot.display_content();
        assert!(display.contains("[Full output:"), "{display}");
        assert!(display.contains("lines shown"), "{display}");
        assert!(display.contains("Truncated:"), "{display}");
        assert!(
            display.contains("/tmp/") || display.contains("(unavailable)"),
            "{display}"
        );
    }

    #[test]
    fn test_display_content_no_truncation() {
        let mut acc = OutputAccumulator::with_limits(1024, 2048);
        acc.append(b"short");
        let snapshot = acc.finish();
        let display = snapshot.display_content();
        assert_eq!(display, "short");
    }
}
