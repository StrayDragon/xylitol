use xylitol_tui::utils::*;

#[test]
fn test_visible_width_ascii() {
    assert_eq!(visible_width("hello"), 5);
    assert_eq!(visible_width(""), 0);
}

#[test]
fn test_visible_width_cjk() {
    // Fullwidth CJK characters - unicode-width determines actual width
    // Depending on unicode-width version, may be 2 or 3 per char
    let w = visible_width("你好");
    assert!(w >= 2);
    // Each CJK char is at least 2 wide
    assert_eq!(visible_width("好"), w / 2);
}

#[test]
fn test_visible_width_tabs() {
    // Tabs are 3 chars wide
    assert_eq!(visible_width("\t"), 3);
    assert_eq!(visible_width("a\tb"), 5);
}

#[test]
fn test_visible_width_ansi() {
    // ANSI colors should be stripped
    assert_eq!(visible_width("\x1b[31mhello\x1b[0m"), 5);
    assert_eq!(visible_width("\x1b[1;32mtest\x1b[0m"), 4);
}

#[test]
fn test_truncate_to_width_ascii() {
    let result = truncate_to_width("hello world", 8, "...", false);
    assert!(result.contains("hello"));
    assert!(result.contains("..."));
    assert!(!result.contains("world"));

    assert_eq!(truncate_to_width("hi", 10, "...", false), "hi");
    let padded = truncate_to_width("hi", 10, "...", true);
    assert!(padded.starts_with("hi"));
    assert!(padded.len() >= 10);
}

#[test]
fn test_truncate_to_width_empty() {
    assert_eq!(truncate_to_width("", 10, "...", false), "");
    assert_eq!(truncate_to_width("", 10, "...", true), "          ");
}

#[test]
fn test_slice_by_column() {
    assert_eq!(slice_by_column("hello", 1, 3), "ell");
    assert_eq!(slice_by_column("hello", 0, 5), "hello");
    assert_eq!(slice_by_column("hello", 0, 0), "");
}

#[test]
fn test_wrap_text_with_ansi_basic() {
    let lines = wrap_text_with_ansi("hello world", 5);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "hello");
    // "world" should be on second line
    assert!(lines[1].contains("world"));
}

#[test]
fn test_wrap_text_with_ansi_empty() {
    let lines = wrap_text_with_ansi("", 10);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0], "");
}

#[test]
fn test_wrap_text_with_ansi_preserves_ansi() {
    let colored = "\x1b[31mred text\x1b[0m";
    let lines = wrap_text_with_ansi(colored, 20);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("\x1b[31m"));
    assert!(lines[0].contains("red"));
}

#[test]
fn test_visible_width_with_osc() {
    // OSC hyperlinks should be stripped
    assert_eq!(
        visible_width("\x1b]8;;https://example.com\x07link\x1b]8;;\x07"),
        4
    );
}

#[test]
fn test_extract_ansi_code_csi() {
    let (code, len) = extract_ansi_code("\x1b[31m", 0).unwrap();
    assert_eq!(code, "\x1b[31m");
    assert_eq!(len, 5);
}

#[test]
fn test_extract_ansi_code_none() {
    assert!(extract_ansi_code("hello", 0).is_none());
}

#[test]
fn test_is_whitespace_char() {
    assert!(is_whitespace_char(' '));
    assert!(is_whitespace_char('\t'));
    assert!(!is_whitespace_char('a'));
}

#[test]
fn test_is_punctuation_char() {
    assert!(is_punctuation_char('.'));
    assert!(is_punctuation_char(','));
    assert!(!is_punctuation_char('a'));
}
