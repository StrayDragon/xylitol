use xylitol_tui::word_navigation::*;

#[test]
fn test_find_word_backward_basic() {
    assert_eq!(find_word_backward("hello world", 6), 0);
    assert_eq!(find_word_backward("hello world", 11), 6);
    assert_eq!(find_word_backward("hello_world", 5), 0);
    assert_eq!(find_word_backward("", 0), 0);
}

#[test]
fn test_find_word_forward_basic() {
    assert_eq!(find_word_forward("hello world", 0), 5);
    assert_eq!(find_word_forward("hello world", 6), 11);
    assert_eq!(find_word_forward("", 0), 0);
    assert_eq!(find_word_forward("hello", 5), 5);
}

#[test]
fn test_find_word_backward_punctuation() {
    assert_eq!(find_word_backward("ab.cd", 4), 3);
}

#[test]
fn test_find_word_forward_punctuation() {
    assert_eq!(find_word_forward("ab.cd", 2), 3);
}

#[test]
fn test_find_word_forward_skip_spaces() {
    assert_eq!(find_word_forward("  hello", 0), 7);
}
