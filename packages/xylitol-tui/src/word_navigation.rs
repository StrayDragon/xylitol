use crate::utils::is_whitespace_char;
use unicode_segmentation::UnicodeSegmentation;

/// Find the cursor position after moving one word backward from `cursor` in `text`.
/// Skips trailing whitespace, then stops at the next word/punctuation boundary.
pub fn find_word_backward(text: &str, cursor: usize) -> usize {
    if cursor == 0 {
        return 0;
    }

    let before = &text[..cursor];
    let graphemes: Vec<&str> = UnicodeSegmentation::graphemes(before, true).collect();
    if graphemes.is_empty() {
        return 0;
    }

    let mut idx = graphemes.len();

    // Skip trailing whitespace
    while idx > 0 {
        let g = graphemes[idx - 1];
        if is_whitespace_char(g.chars().next().unwrap_or(' ')) {
            idx -= 1;
        } else {
            break;
        }
    }

    if idx == 0 {
        return 0;
    }

    // Determine the type of the last non-whitespace: word-like or punctuation
    let last_g = graphemes[idx - 1];
    let last_ch = last_g.chars().next().unwrap();
    let is_word = last_ch.is_alphanumeric() || last_ch == '_';

    if is_word {
        // Inside a word: skip to start of that word
        while idx > 0 {
            let g = graphemes[idx - 1];
            let ch = g.chars().next().unwrap();
            if is_whitespace_char(ch) || is_punctuation_ch(ch) {
                break;
            }
            idx -= 1;
        }
    } else {
        // In punctuation: skip all contiguous punctuation
        while idx > 0 {
            let g = graphemes[idx - 1];
            let ch = g.chars().next().unwrap();
            if is_whitespace_char(ch) || (ch.is_alphanumeric() || ch == '_') {
                break;
            }
            idx -= 1;
        }
    }

    graphemes[..idx].iter().map(|g| g.len()).sum()
}

/// Find the cursor position after moving one word forward from `cursor` in `text`.
/// Skips leading whitespace, then stops at the next word/punctuation boundary.
pub fn find_word_forward(text: &str, cursor: usize) -> usize {
    if cursor >= text.len() {
        return text.len();
    }

    let after = &text[cursor..];
    let graphemes: Vec<&str> = UnicodeSegmentation::graphemes(after, true).collect();
    if graphemes.is_empty() {
        return cursor;
    }

    let mut idx = 0usize;

    // Skip leading whitespace
    while idx < graphemes.len() {
        let g = graphemes[idx];
        if is_whitespace_char(g.chars().next().unwrap_or('a')) {
            idx += 1;
        } else {
            break;
        }
    }

    if idx >= graphemes.len() {
        return cursor + graphemes.iter().map(|g| g.len()).sum::<usize>();
    }

    let first_g = graphemes[idx];
    let first_ch = first_g.chars().next().unwrap();
    let is_word = first_ch.is_alphanumeric() || first_ch == '_';

    if is_word {
        // Inside a word: skip to end of that word (or punctuation within)
        while idx < graphemes.len() {
            let g = graphemes[idx];
            let ch = g.chars().next().unwrap();
            if is_whitespace_char(ch) || is_punctuation_ch(ch) {
                break;
            }
            idx += 1;
        }
    } else {
        // In punctuation: skip all contiguous punctuation
        while idx < graphemes.len() {
            let g = graphemes[idx];
            let ch = g.chars().next().unwrap();
            if is_whitespace_char(ch) || ch.is_alphanumeric() || ch == '_' {
                break;
            }
            idx += 1;
        }
    }

    cursor + graphemes[..idx].iter().map(|g| g.len()).sum::<usize>()
}

fn is_punctuation_ch(ch: char) -> bool {
    crate::utils::is_punctuation_char(ch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_word_backward_basic() {
        assert_eq!(find_word_backward("hello world", 6), 0); // "hello " -> back to word start
        assert_eq!(find_word_backward("hello world", 11), 6); // past "world" -> 6
        assert_eq!(find_word_backward("hello_world", 5), 0);
    }

    #[test]
    fn test_find_word_forward_basic() {
        assert_eq!(find_word_forward("hello world", 0), 5); // "hello"
        assert_eq!(find_word_forward("hello world", 6), 11); // into "world"
    }

    #[test]
    fn test_punctuation_boundary() {
        assert_eq!(find_word_backward("ab.cd", 4), 3); // skip over '.', back to 'cd'
        assert_eq!(find_word_forward("ab.cd", 2), 3); // stop at '.'
    }
}
