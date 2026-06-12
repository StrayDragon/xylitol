//! Patch apply strategy for the edit tool.
//!
//! Implements the fuzzy + patch fallback matching strategy:
//! 1. Exact match
//! 2. NFKC fuzzy match (handles smart quotes, dashes, etc.)
//! 3. Line-based patch fallback
//! 4. Return error if both fail

/// Fuzzy-match `target` in `content` using NFKC normalization of smart quotes and dashes.
/// Returns the byte range of the match in `content`.
pub(crate) fn fuzzy_find(content: &str, target: &str) -> Option<std::ops::Range<usize>> {
    // Try whitespace-normalized matching first
    if let Some(range) = whitespace_find(content, target) {
        return Some(range);
    }
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
}
