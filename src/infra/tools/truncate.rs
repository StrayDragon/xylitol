//! Shared truncation utilities for tool outputs.
//!
//! Truncation is based on two independent limits — whichever is hit first wins:
//! - Line limit (default: 2000 lines)
//! - Byte limit (default: 50KB)
//!
//! Never returns partial lines (except bash tail truncation edge case).

pub(crate) const DEFAULT_MAX_LINES: usize = 2000;
pub(crate) const DEFAULT_MAX_BYTES: usize = 50 * 1024; // 50KB
pub(crate) const GREP_MAX_LINE_LENGTH: usize = 500; // Max chars per grep match line

/// Result of a truncation operation.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct TruncationResult {
    /// The truncated content.
    pub(crate) content: String,
    /// Whether truncation occurred.
    pub(crate) truncated: bool,
    /// Which limit was hit: "lines", "bytes", or None if not truncated.
    pub(crate) truncated_by: Option<TruncationLimit>,
    /// Total number of lines in the original content.
    pub(crate) total_lines: usize,
    /// Total number of bytes in the original content.
    pub(crate) total_bytes: usize,
    /// Number of complete lines in the truncated output.
    pub(crate) output_lines: usize,
    /// Number of bytes in the truncated output.
    pub(crate) output_bytes: usize,
    /// Whether the first line was partially truncated (for head truncation when first line > max bytes).
    pub(crate) last_line_partial: bool,
    /// Whether the first line alone exceeded the byte limit.
    pub(crate) first_line_exceeds_limit: bool,
    /// The max lines limit that was applied.
    pub(crate) max_lines: usize,
    /// The max bytes limit that was applied.
    pub(crate) max_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TruncationLimit {
    Lines,
    Bytes,
}

impl std::fmt::Display for TruncationLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TruncationLimit::Lines => write!(f, "lines"),
            TruncationLimit::Bytes => write!(f, "bytes"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TruncationOptions {
    pub(crate) max_lines: Option<usize>,
    pub(crate) max_bytes: Option<usize>,
}

/// Split content into lines for counting (excluding trailing empty line from final \n).
fn split_lines_for_counting(content: &str) -> Vec<&str> {
    if content.is_empty() {
        return vec![];
    }
    let mut lines: Vec<&str> = content.split('\n').collect();
    if content.ends_with('\n') {
        lines.pop();
    }
    lines
}

/// Truncate content from the head (keep first N lines/bytes).
/// Suitable for file reads where you want to see the beginning.
///
/// Never returns partial lines. If first line exceeds byte limit,
/// returns empty content with `first_line_exceeds_limit = true`.
pub(crate) fn truncate_head(content: &str, options: TruncationOptions) -> TruncationResult {
    let max_lines = options.max_lines.unwrap_or(DEFAULT_MAX_LINES);
    let max_bytes = options.max_bytes.unwrap_or(DEFAULT_MAX_BYTES);

    let total_bytes = content.len();
    let lines = split_lines_for_counting(content);
    let total_lines = lines.len();

    // Check if no truncation needed
    if total_lines <= max_lines && total_bytes <= max_bytes {
        return TruncationResult {
            content: content.to_string(),
            truncated: false,
            truncated_by: None,
            total_lines,
            total_bytes,
            output_lines: total_lines,
            output_bytes: total_bytes,
            last_line_partial: false,
            first_line_exceeds_limit: false,
            max_lines,
            max_bytes,
        };
    }

    // Check if first line alone exceeds byte limit
    if let Some(first) = lines.first()
        && first.len() > max_bytes
    {
        return TruncationResult {
            content: String::new(),
            truncated: true,
            truncated_by: Some(TruncationLimit::Bytes),
            total_lines,
            total_bytes,
            output_lines: 0,
            output_bytes: 0,
            last_line_partial: false,
            first_line_exceeds_limit: true,
            max_lines,
            max_bytes,
        };
    }

    // Collect complete lines that fit
    let mut output_lines_arr: Vec<&str> = Vec::new();
    let mut output_bytes_count = 0usize;
    let mut truncated_by = TruncationLimit::Lines;

    for (i, line) in lines.iter().enumerate() {
        if i >= max_lines {
            break;
        }
        let line_bytes = line.len() + if i > 0 { 1 } else { 0 }; // +1 for newline

        if output_bytes_count + line_bytes > max_bytes {
            truncated_by = TruncationLimit::Bytes;
            break;
        }

        output_lines_arr.push(line);
        output_bytes_count += line_bytes;
    }

    // If we exited due to line limit and still within bytes
    if output_lines_arr.len() >= max_lines && output_bytes_count <= max_bytes {
        truncated_by = TruncationLimit::Lines;
    }

    let output_content = output_lines_arr.join("\n");
    let final_output_bytes = output_content.len();

    TruncationResult {
        content: output_content,
        truncated: true,
        truncated_by: Some(truncated_by),
        total_lines,
        total_bytes,
        output_lines: output_lines_arr.len(),
        output_bytes: final_output_bytes,
        last_line_partial: false,
        first_line_exceeds_limit: false,
        max_lines,
        max_bytes,
    }
}
/// Truncate a single line to max characters, adding [truncated] suffix.
/// Used for grep match lines.
pub(crate) fn truncate_line(line: &str, max_chars: usize) -> TruncatedLine {
    if line.char_indices().count() <= max_chars {
        return TruncatedLine {
            text: line.to_string(),
            was_truncated: false,
        };
    }

    let char_count = line.char_indices().count();
    if char_count <= max_chars {
        return TruncatedLine {
            text: line.to_string(),
            was_truncated: false,
        };
    }

    // Take first max_chars characters
    let truncated: String = line
        .chars()
        .take(max_chars)
        .chain("... [truncated]".chars())
        .collect();

    TruncatedLine {
        text: truncated,
        was_truncated: true,
    }
}

pub(crate) struct TruncatedLine {
    pub(crate) text: String,
    #[allow(dead_code)]
    pub(crate) was_truncated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_truncation_needed() {
        let content = "short\n";
        let result = truncate_head(content, TruncationOptions::default());
        assert!(!result.truncated);
        assert_eq!(result.content, content);
        assert_eq!(result.total_lines, 1);
    }

    #[test]
    fn test_truncate_head_lines() {
        let lines: Vec<String> = (0..5000).map(|i| format!("line {i}")).collect();
        let content = lines.join("\n");
        let result = truncate_head(&content, TruncationOptions::default());
        assert!(result.truncated);
        assert_eq!(result.truncated_by, Some(TruncationLimit::Lines));
        assert!(result.output_lines <= DEFAULT_MAX_LINES);
    }

    #[test]
    fn test_truncate_head_first_line_too_large() {
        let very_long = "x".repeat(100_000);
        let result = truncate_head(&very_long, TruncationOptions::default());
        assert!(result.truncated);
        assert!(result.first_line_exceeds_limit);
        assert_eq!(result.content, "");
    }

    #[test]
    fn test_truncate_line_within_limit() {
        let result = truncate_line("short line", GREP_MAX_LINE_LENGTH);
        assert!(!result.was_truncated);
        assert_eq!(result.text, "short line");
    }

    #[test]
    fn test_truncate_line_exceeds_limit() {
        let long = "x".repeat(600);
        let result = truncate_line(&long, GREP_MAX_LINE_LENGTH);
        assert!(result.was_truncated);
        assert!(result.text.ends_with("[truncated]"));
    }
}
