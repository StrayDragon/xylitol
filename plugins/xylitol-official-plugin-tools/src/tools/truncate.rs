pub(crate) const DEFAULT_MAX_LINES: usize = 2000;
pub(crate) const DEFAULT_MAX_BYTES: usize = 50 * 1024;
pub(crate) const GREP_MAX_LINE_LENGTH: usize = 500;

#[derive(Debug, Clone)]
pub(crate) struct TruncationResult {
    pub(crate) content: String,
    pub(crate) truncated: bool,
    pub(crate) truncated_by: Option<TruncationLimit>,
    pub(crate) output_lines: usize,
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

pub(crate) fn truncate_head(content: &str, options: TruncationOptions) -> TruncationResult {
    let max_lines = options.max_lines.unwrap_or(DEFAULT_MAX_LINES);
    let max_bytes = options.max_bytes.unwrap_or(DEFAULT_MAX_BYTES);

    let total_bytes = content.len();
    let lines = split_lines_for_counting(content);
    let total_lines = lines.len();

    if total_lines <= max_lines && total_bytes <= max_bytes {
        return TruncationResult {
            content: content.to_string(),
            truncated: false,
            truncated_by: None,
            output_lines: total_lines,
        };
    }

    if let Some(first) = lines.first()
        && first.len() > max_bytes
    {
        return TruncationResult {
            content: String::new(),
            truncated: true,
            truncated_by: Some(TruncationLimit::Bytes),
            output_lines: 0,
        };
    }

    let mut output_lines_arr: Vec<&str> = Vec::new();
    let mut output_bytes_count = 0usize;
    let mut truncated_by = TruncationLimit::Lines;

    for (i, line) in lines.iter().enumerate() {
        if i >= max_lines {
            break;
        }
        let line_bytes = line.len() + if i > 0 { 1 } else { 0 };

        if output_bytes_count + line_bytes > max_bytes {
            truncated_by = TruncationLimit::Bytes;
            break;
        }

        output_lines_arr.push(line);
        output_bytes_count += line_bytes;
    }

    if output_lines_arr.len() >= max_lines && output_bytes_count <= max_bytes {
        truncated_by = TruncationLimit::Lines;
    }

    let output_content = output_lines_arr.join("\n");

    TruncationResult {
        content: output_content,
        truncated: true,
        truncated_by: Some(truncated_by),
        output_lines: output_lines_arr.len(),
    }
}
pub(crate) fn truncate_line(line: &str, max_chars: usize) -> TruncatedLine {
    if line.char_indices().count() <= max_chars {
        return TruncatedLine {
            text: line.to_string(),
        };
    }

    let truncated: String = line
        .chars()
        .take(max_chars)
        .chain("... [truncated]".chars())
        .collect();

    TruncatedLine { text: truncated }
}

pub(crate) struct TruncatedLine {
    pub(crate) text: String,
}

pub(crate) fn push_limit_notices(out: &mut String, extra: Vec<String>, truncated: bool) {
    let mut notices = extra;
    if truncated {
        notices.push(format!(
            "{} limit reached",
            crate::text::format_size(DEFAULT_MAX_BYTES as u64)
        ));
    }
    if !notices.is_empty() {
        out.push_str(&format!("\n\n[{}]", notices.join(". ")));
    }
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
        assert_eq!(result.output_lines, 1);
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
        assert_eq!(result.truncated_by, Some(TruncationLimit::Bytes));
        assert_eq!(result.content, "");
    }

    #[test]
    fn test_truncate_line_within_limit() {
        let result = truncate_line("short line", GREP_MAX_LINE_LENGTH);
        assert_eq!(result.text, "short line");
    }

    #[test]
    fn test_truncate_line_exceeds_limit() {
        let long = "x".repeat(600);
        let result = truncate_line(&long, GREP_MAX_LINE_LENGTH);
        assert!(result.text.ends_with("[truncated]"));
    }
}
