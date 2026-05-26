//! Patch apply strategy for the edit tool.
//!
//! Implements the fudiff + patch fallback strategy:
//! 1. Try fudiff (fuzzy matching) when exact match fails
//! 2. Fall back to patch (exact line-based matching)
//! 3. Return error if both fail

/// Fuzzy-replace `old_string` with `new_string` in `content`.
///
/// Uses whitespace-normalized matching to find the target when exact
/// match fails due to minor formatting differences. Returns the modified
/// content when a fuzzy match is found, or `None` when the confidence
/// is too low.
pub(crate) fn fudiff_replace(content: &str, old_string: &str, new_string: &str) -> Option<String> {
    let normalized_old = normalize_ws(old_string);
    let normalized_content = normalize_ws(content);

    if !normalized_content.contains(&normalized_old) {
        return None;
    }

    // Find the exact match position in normalized content
    if let Some(norm_pos) = normalized_content.find(&normalized_old) {
        // Count characters (in normalized content) before match
        let pre_normalized = &normalized_content[..norm_pos];
        let pre_chars = pre_normalized.chars().count();

        let mut pos_in_content = 0;
        for (char_count, ch) in content.chars().enumerate() {
            if char_count >= pre_chars {
                break;
            }
            pos_in_content += ch.len_utf8();
        }

        // Now try to find old_string at this approximate position
        // by checking variations
        let search_window = &content[pos_in_content..];
        if let Some(found) = find_best_match(search_window, old_string) {
            let modified = format!(
                "{}{}{}",
                &content[..pos_in_content + found.start],
                new_string,
                &content[pos_in_content + found.end..]
            );
            return Some(modified);
        }

        // Last resort: use normalized position directly
        // Find the text from content that normalized to old_string and replace it
        let old_trimmed = old_string.trim();
        if let Some(pos) = content.find(old_trimmed) {
            let modified = format!(
                "{}{}{}",
                &content[..pos],
                new_string,
                &content[pos + old_trimmed.len()..]
            );
            return Some(modified);
        }
    }

    None
}

fn normalize_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn lines_match(a: &str, b: &str) -> bool {
    a.trim() == b.trim() || normalize_ws(a) == normalize_ws(b)
}

/// Find the best match for `target` within `text`, considering minor variations.
/// Returns the byte range of the best match.
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

/// Fall back to patch-like exact line-by-line matching.
/// This is a simplified patch strategy that matches exact lines.
pub(crate) fn patch_fallback(content: &str, old_string: &str, new_string: &str) -> Option<String> {
    let old_lines: Vec<&str> = old_string.lines().collect();
    let content_lines: Vec<&str> = content.lines().collect();
    let has_trailing_newline = content.ends_with('\n');

    if old_lines.is_empty() || content_lines.is_empty() {
        return None;
    }

    for start_idx in 0..content_lines
        .len()
        .saturating_sub(old_lines.len().saturating_sub(1))
    {
        let mut match_found = true;
        for (j, old_line) in old_lines.iter().enumerate() {
            if !lines_match(content_lines[start_idx + j], old_line) {
                match_found = false;
                break;
            }
        }

        if match_found {
            let mut result: Vec<&str> = Vec::new();
            result.extend_from_slice(&content_lines[..start_idx]);
            for new_line in new_string.lines() {
                result.push(new_line);
            }
            result.extend_from_slice(&content_lines[start_idx + old_lines.len()..]);

            let mut result_str = result.join("\n");
            if has_trailing_newline {
                result_str.push('\n');
            }
            return Some(result_str);
        }
    }

    None
}

/// Generate a simple human-readable diff between old and new strings.
pub(crate) fn generate_diff(old_string: &str, new_string: &str) -> String {
    use similar::{ChangeTag, TextDiff};

    if old_string == new_string {
        return "(no changes)".to_string();
    }

    let diff = TextDiff::from_lines(old_string, new_string);
    let mut output = String::new();

    for change in diff.iter_all_changes() {
        let prefix = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        output.push_str(prefix);
        output.push_str(change.value());
        if !change.value().ends_with('\n') {
            output.push('\n');
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fudiff_exact_match() {
        let content = "hello world\nfoo bar\nbaz qux\n";
        let result = fudiff_replace(content, "foo bar", "new foo");
        assert_eq!(result, Some("hello world\nnew foo\nbaz qux\n".to_string()));
    }

    #[test]
    fn test_fudiff_whitespace_normalized() {
        let content = "hello   world\n";
        let result = fudiff_replace(content, "hello world", "hi world");
        assert_eq!(result, Some("hi world\n".to_string()));
    }

    #[test]
    fn test_fudiff_no_match() {
        let content = "hello world\n";
        let result = fudiff_replace(content, "completely different", "nothing");
        assert_eq!(result, None);
    }

    #[test]
    fn test_patch_fallback_exact() {
        let content = "line1\nline2\nline3\n";
        let result = patch_fallback(content, "line2", "replaced");
        assert_eq!(result, Some("line1\nreplaced\nline3\n".to_string()));
    }

    #[test]
    fn test_patch_fallback_trimmed_match() {
        let content = "line1\n  line2  \nline3\n";
        let result = patch_fallback(content, "line2", "replaced");
        assert_eq!(result, Some("line1\nreplaced\nline3\n".to_string()));
    }

    #[test]
    fn test_patch_fallback_no_match() {
        let content = "hello world\n";
        let result = patch_fallback(content, "goodbye", "nothing");
        assert_eq!(result, None);
    }

    #[test]
    fn test_generate_diff_shows_changes() {
        let diff = generate_diff("old text", "new text");
        assert!(diff.contains("-old text"));
        assert!(diff.contains("+new text"));
    }

    #[test]
    fn test_generate_diff_no_changes() {
        let diff = generate_diff("same", "same");
        assert_eq!(diff, "(no changes)");
    }

    #[test]
    fn test_fudiff_multiline() {
        let content = "a\nb\nc\nd\ne\n";
        let result = fudiff_replace(content, "b\nc\nd", "x\ny\nz");
        assert_eq!(result, Some("a\nx\ny\nz\ne\n".to_string()));
    }
}
