use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use super::truncate::DEFAULT_MAX_BYTES;

const DEFAULT_MAX_ROLLING_BYTES_FACTOR: usize = 2;

pub(crate) struct OutputAccumulator {
    max_bytes: usize,
    max_rolling_bytes: usize,
    rolling_text: String,
    rolling_bytes: usize,
    temp_file: Option<PathBuf>,
    temp_writer: Option<BufWriter<File>>,
    total_bytes: usize,
    pre_chunks: Vec<Vec<u8>>,
    spilled_to_file: bool,
}

impl Default for OutputAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputAccumulator {
    pub fn new() -> Self {
        Self::with_limits(
            DEFAULT_MAX_BYTES,
            DEFAULT_MAX_BYTES * DEFAULT_MAX_ROLLING_BYTES_FACTOR,
        )
    }

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

    pub fn append(&mut self, data: &[u8]) {
        self.total_bytes += data.len();

        if self.spilled_to_file {
            if let Some(ref mut writer) = self.temp_writer {
                let _ = writer.write_all(data);
            }
            self.append_rolling(data);
        } else {
            let current_total =
                self.pre_chunks.iter().map(|c| c.len()).sum::<usize>() + self.pre_chunks.len();

            if current_total + data.len() > self.max_rolling_bytes {
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

    fn append_rolling(&mut self, data: &[u8]) {
        let text = String::from_utf8_lossy(data);
        self.rolling_bytes += text.len();
        self.rolling_text.push_str(&text);

        if self.rolling_bytes > self.max_bytes {
            let excess = self.rolling_bytes - self.max_bytes;
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

    fn spill_to_file(&mut self) {
        if self.spilled_to_file {
            return;
        }

        let temp_path =
            std::env::temp_dir().join(format!("xylitol-output-{}.txt", uuid::Uuid::new_v4()));

        if let Ok(file) = File::create(&temp_path) {
            let mut writer = BufWriter::new(file);

            for chunk in &self.pre_chunks {
                let _ = writer.write_all(chunk);
            }

            let _ = writer.flush();

            self.temp_file = Some(temp_path);
            self.temp_writer = Some(writer);
            self.spilled_to_file = true;
            self.pre_chunks.clear();
        }
    }

    pub fn finish(&mut self) -> OutputSnapshot {
        if let Some(mut writer) = self.temp_writer.take() {
            let _ = writer.flush();
        }

        let truncated = self.total_bytes > self.max_bytes;

        OutputSnapshot {
            content: self.rolling_text.clone(),
            truncated,
            max_bytes: self.max_bytes,
            full_output_path: self.temp_file.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct OutputSnapshot {
    pub(crate) content: String,
    pub(crate) truncated: bool,
    pub(crate) max_bytes: usize,
    pub(crate) full_output_path: Option<PathBuf>,
}

impl OutputSnapshot {
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
            crate::text::format_size(self.max_bytes as u64),
        )
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
    }

    #[test]
    fn test_overflow_creates_temp_file() {
        let mut acc = OutputAccumulator::with_limits(100, 200);
        let data = vec![b'a'; 300];
        acc.append(&data);
        let snapshot = acc.finish();
        assert!(snapshot.truncated);
        assert!(snapshot.full_output_path.is_some());
    }

    #[test]
    fn test_rolling_buffer_tail() {
        let mut acc = OutputAccumulator::with_limits(50, 100);
        acc.append(b"123456789012345678901234567890");
        acc.append(b"123456789012345678901234567890");
        acc.append(b"123456789012345678901234567890");
        let snapshot = acc.finish();

        assert!(snapshot.content.len() <= 50);
    }

    #[test]
    fn test_multiple_appends() {
        let mut acc = OutputAccumulator::with_limits(1024, 2048);
        acc.append(b"line1\n");
        acc.append(b"line2\n");
        acc.append(b"line3\n");
        let snapshot = acc.finish();
        assert_eq!(snapshot.content, "line1\nline2\nline3\n");
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
        let path = snapshot
            .full_output_path
            .as_ref()
            .and_then(|p| p.to_str())
            .expect("truncated snapshot carries the full-output path");
        assert!(display.contains(path), "{display}");
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
