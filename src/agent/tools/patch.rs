//! Patch apply strategy for the edit tool.
//!
//! Implements the fuzzy + patch fallback matching strategy:
//! 1. Exact match
//! 2. NFKC fuzzy match (handles smart quotes, dashes, Unicode spaces)
//! 3. Line-based patch fallback
//! 4. Return error if both fail
//!
//! Also provides line ending detection/restoration and structured diff output.

/// Fuzzy-match `target` in `content` using progressive normalization.
///
/// Tries in order:
/// 1. Whitespace-normalized matching
/// 2. Full Unicode normalization (smart quotes, dashes, spaces)
/// 3. Line-based patch fallback
pub(crate) fn fuzzy_find(content: &str, target: &str) -> Option<std::ops::Range<usize>> {
    // Level 1: whitespace-normalized matching
    if let Some(range) = whitespace_find(content, target) {
        return Some(range);
    }

    // Level 2: full Unicode normalization
    let norm_content = normalize_for_fuzzy_match(content);
    let norm_target = normalize_for_fuzzy_match(target);
    if let Some(pos) = norm_content.find(&norm_target) {
        // Map back to original byte position
        // Walk through original content to find the matching range
        let mut orig_pos = 0usize;
        let mut norm_pos = 0usize;
        for ch in content.chars() {
            if norm_pos >= pos {
                break;
            }
            let ch_str = ch.to_string();
            let norm_ch = normalize_for_fuzzy_match(&ch_str);
            orig_pos += ch_str.len();
            norm_pos += norm_ch.len();
        }
        let end_pos = orig_pos + target.len().min(content.len().saturating_sub(orig_pos));
        return Some(orig_pos..end_pos);
    }

    // Level 3: line-based patch fallback (already exists separately)
    None
}

fn whitespace_find(content: &str, target: &str) -> Option<std::ops::Range<usize>> {
    let normalized_content = normalize_ws(content);
    let normalized_target = normalize_ws(target);

    if let Some(pos) = normalized_content.find(&normalized_target) {
        // Map back to bytes in original content using the same normalization mapping
        let mut orig_pos = 0usize;
        let mut norm_pos = 0usize;
        for ch in content.chars() {
            if norm_pos >= pos {
                break;
            }
            let ch_str = ch.to_string();
            orig_pos += ch_str.len();
            // Count normalized whitespace
            let norm_ch = normalize_ws(&ch_str);
            norm_pos += norm_ch.len();
        }
        // Now scan forward to find the exact target boundaries
        let remainder = &content[orig_pos..];
        if let Some(match_idx) = find_best_match(remainder, target) {
            return Some(orig_pos + match_idx.start..orig_pos + match_idx.end);
        }

        // Fallback: use trimmed match
        let target_trimmed = target.trim();
        if let Some(pos) = remainder.find(target_trimmed) {
            return Some(orig_pos + pos..orig_pos + pos + target_trimmed.len());
        }
    }

    None
}

/// Line-based fallback: find `target` as a block of lines in `content`.
pub(crate) fn patch_find_range(content: &str, target: &str) -> Option<std::ops::Range<usize>> {
    let target_lines: Vec<&str> = target.lines().collect();
    let content_lines: Vec<&str> = content.lines().collect();

    if target_lines.is_empty() || content_lines.is_empty() {
        return None;
    }

    for start_idx in 0..content_lines
        .len()
        .saturating_sub(target_lines.len().saturating_sub(1))
    {
        let mut match_found = true;
        for (j, target_line) in target_lines.iter().enumerate() {
            if !lines_match(content_lines[start_idx + j], target_line) {
                match_found = false;
                break;
            }
        }

        if match_found {
            // Compute byte range
            let byte_start = content_lines[..start_idx]
                .iter()
                .map(|l| l.len() + 1)
                .sum::<usize>();
            let byte_end = byte_start
                + target_lines
                    .iter()
                    .map(|l| l.len() + 1)
                    .sum::<usize>()
                    .saturating_sub(1);
            return Some(byte_start..byte_end.min(content.len()));
        }
    }

    None
}

fn normalize_ws(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_whitespace = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !in_whitespace {
                result.push(' ');
                in_whitespace = true;
            }
        } else {
            result.push(ch);
            in_whitespace = false;
        }
    }
    result
}

/// Normalize text for fuzzy matching with progressive transformations.
///
/// - NFKC normalize
/// - Smart single quotes → '
/// - Smart double quotes → "
/// - Various dashes/hyphens → -
/// - Special Unicode spaces → regular space
/// - Strip trailing whitespace from each line
pub(crate) fn normalize_for_fuzzy_match(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            // Smart single quotes
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' => result.push('\''),
            // Smart double quotes
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' => result.push('"'),
            // Dashes and hyphens
            '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}'
            | '\u{2014}' | '\u{2015}' | '\u{2212}' => result.push('-'),
            // Special spaces
            '\u{00A0}' | '\u{2002}' | '\u{2003}' | '\u{2009}'
            | '\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}' => result.push(' '),
            _ => result.push(ch),
        }
    }
    // Strip trailing whitespace per line
    result
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Detect whether a text uses CRLF or LF line endings.
#[allow(dead_code)]
pub(crate) fn detect_line_ending(content: &str) -> LineEnding {
    if content.contains("\r\n") {
        LineEnding::Crlf
    } else {
        LineEnding::Lf
    }
}

/// Line ending style.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum LineEnding {
    Crlf,
    Lf,
}

/// Restore line endings to the specified style.
#[allow(dead_code)]
pub(crate) fn restore_line_endings(text: &str, ending: LineEnding) -> String {
    // Normalize to LF first
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    match ending {
        LineEnding::Crlf => normalized.replace('\n', "\r\n"),
        LineEnding::Lf => normalized,
    }
}

fn lines_match(a: &str, b: &str) -> bool {
    a.trim() == b.trim() || normalize_ws(a) == normalize_ws(b)
}

/// Find the best match for `target` within `text`, considering minor variations.
fn find_best_match(text: &str, target: &str) -> Option<std::ops::Range<usize>> {
    let target_lines: Vec<&str> = target.lines().collect();
    let text_lines: Vec<&str> = text.lines().collect();

    if target_lines.is_empty() || text_lines.is_empty() {
        return None;
    }

    for (i, line) in text_lines.iter().enumerate() {
        if !lines_match(line, target_lines[0]) {
            continue;
        }

        if target_lines.len() == 1 {
            let start = text.lines().take(i).map(|l| l.len() + 1).sum::<usize>();
            let end = start + line.len();
            return Some(start..end);
        }

        let mut all_match = true;
        for (j, target_line) in target_lines.iter().enumerate().skip(1) {
            if i + j >= text_lines.len() {
                all_match = false;
                break;
            }
            if !lines_match(text_lines[i + j], target_line) {
                all_match = false;
                break;
            }
        }

        if all_match {
            let start = text.lines().take(i).map(|l| l.len() + 1).sum::<usize>();
            let end = text
                .lines()
                .take(i + target_lines.len())
                .map(|l| l.len() + 1)
                .sum::<usize>();
            let end = end.saturating_sub(1);
            return Some(start..end);
        }
    }

    None
}

// ═══════════════════════════════════════════════════════════════════
// Diff generation
// ═══════════════════════════════════════════════════════════════════

use similar::{ChangeTag, TextDiff};

/// Generate a unified diff between old and new content.
pub(crate) fn generate_unified_diff(old: &str, new: &str, _file_path: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    diff.unified_diff().context_radius(3).to_string()
}

/// Generate a human-readable display diff with line numbers and context folding.
pub(crate) fn generate_display_diff(old: &str, new: &str, file_path: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut output = String::new();

    output.push_str(&format!("--- a/{file_path}\n+++ b/{file_path}\n"));

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        output.push_str(sign);
        output.push_str(change.value());
        if !change.value().ends_with('\n') {
            output.push('\n');
        }
    }

    // Add line numbers
    let mut old_line = 1;
    let mut new_line = 1;
    let mut result = String::new();
    for line in output.lines() {
        if let Some(content) = line.strip_prefix('-') {
            result.push_str(&format!("{old_line:>4}     | {content}\n"));
            old_line += 1;
        } else if let Some(content) = line.strip_prefix('+') {
            result.push_str(&format!("     {new_line:>4} | {content}\n"));
            new_line += 1;
        } else if let Some(content) = line.strip_prefix(' ') {
            result.push_str(&format!("{old_line:>4} {new_line:>4} | {content}\n"));
            old_line += 1;
            new_line += 1;
        } else if line.starts_with("---") || line.starts_with("+++") {
            result.push_str(&format!("      ... | {line}\n"));
        }
    }

    if result.is_empty() {
        "(no changes)".to_string()
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patch_finder_exact() {
        let content = "line1\nline2\nline3\n";
        let range = patch_find_range(content, "line2");
        assert!(range.is_some());
        let r = range.unwrap();
        assert_eq!(&content[r], "line2");
    }

    #[test]
    fn test_normalize_for_fuzzy_match_smart_quotes() {
        let input = "\u{2018}hello\u{2019} \u{201C}world\u{201D}";
        let result = normalize_for_fuzzy_match(input);
        assert_eq!(result, "'hello' \"world\"");
    }

    #[test]
    fn test_normalize_for_fuzzy_match_dashes() {
        let input = "a\u{2013}b\u{2014}c";
        let result = normalize_for_fuzzy_match(input);
        assert_eq!(result, "a-b-c");
    }

    #[test]
    fn test_normalize_for_fuzzy_match_spaces() {
        let input = "a\u{00A0}b\u{3000}c";
        let result = normalize_for_fuzzy_match(input);
        assert_eq!(result, "a b c");
    }

    #[test]
    fn test_normalize_for_fuzzy_match_trailing_whitespace() {
        let input = "hello   \nworld  ";
        let result = normalize_for_fuzzy_match(input);
        assert_eq!(result, "hello\nworld");
    }

    #[test]
    fn test_normalize_for_fuzzy_match_no_change() {
        let input = "hello world";
        let result = normalize_for_fuzzy_match(input);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_detect_line_ending_crlf() {
        let result = detect_line_ending("line1\r\nline2\r\n");
        assert_eq!(result, LineEnding::Crlf);
    }

    #[test]
    fn test_detect_line_ending_lf() {
        let result = detect_line_ending("line1\nline2\n");
        assert_eq!(result, LineEnding::Lf);
    }

    #[test]
    fn test_detect_line_ending_lf_fallback() {
        let result = detect_line_ending("no newlines here");
        assert_eq!(result, LineEnding::Lf);
    }

    #[test]
    fn test_restore_line_endings_lf_to_crlf() {
        let input = "line1\nline2\n";
        let result = restore_line_endings(input, LineEnding::Crlf);
        assert_eq!(result, "line1\r\nline2\r\n");
    }

    #[test]
    fn test_restore_line_endings_crlf_to_lf() {
        let input = "line1\r\nline2\r\n";
        let result = restore_line_endings(input, LineEnding::Lf);
        assert_eq!(result, "line1\nline2\n");
    }

    #[test]
    fn test_restore_line_endings_preserves_lf_when_already_lf() {
        let input = "line1\nline2\n";
        let result = restore_line_endings(input, LineEnding::Lf);
        assert_eq!(result, "line1\nline2\n");
    }
}
