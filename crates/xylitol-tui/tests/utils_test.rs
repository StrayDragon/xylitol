use xylitol_tui::utils::*;

#[test]
fn test_visible_width_ascii() {
    assert_eq!(visible_width("hello"), 5);
    assert_eq!(visible_width(""), 0);
}

#[test]
fn test_visible_width_cjk() {
    let w = visible_width("你好");
    assert!(w >= 2);
    assert_eq!(visible_width("好"), w / 2);
}

#[test]
fn test_visible_width_tabs() {
    assert_eq!(visible_width("\t"), 3);
    assert_eq!(visible_width("a\tb"), 5);
}

#[test]
fn test_visible_width_ansi() {
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
fn test_wrap_ansi_ascii_matches_plain_breaks() {
    let plain = "hello world from wrap fast path test case";
    let styled = format!("\x1b[32m{plain}\x1b[0m");
    let a = wrap_text_with_ansi(plain, 12);
    let b = wrap_text_with_ansi(&styled, 12);
    assert!(a.len() > 1, "expected wrap: {a:?}");
    assert_eq!(a.len(), b.len(), "plain={a:?} styled={b:?}");
    for (pa, pb) in a.iter().zip(b.iter()) {
        let strip = |s: &str| {
            let mut out = String::new();
            let mut i = 0;
            let bytes = s.as_bytes();
            while i < bytes.len() {
                if bytes[i] == 0x1b
                    && let Some((_, len)) = extract_ansi_code(&s[i..], 0)
                {
                    i += len;
                    continue;
                }
                out.push(s[i..].chars().next().unwrap());
                i += s[i..].chars().next().unwrap().len_utf8();
            }
            out
        };
        assert_eq!(
            strip(pa),
            strip(pb),
            "line mismatch plain={pa:?} styled={pb:?}"
        );
    }
}

#[test]
fn test_wrap_cjk_still_breaks_per_cluster() {
    let lines = wrap_text_with_ansi("你好世界测试", 4);
    assert!(lines.len() >= 2, "CJK should wrap: {lines:?}");
    assert!(lines.iter().all(|l| visible_width(l) <= 4), "{lines:?}");
}

#[test]
fn test_visible_width_with_osc() {
    assert_eq!(
        visible_width("\x1b]8;;https://example.com\x07link\x1b]8;;\x07"),
        4
    );
}

#[test]
fn test_visible_width_ansi_ascii_matches_plain() {
    let plain = "Resume Session (Current Folder)  Current | All";
    let styled = format!("\x1b[38;5;245m{plain}\x1b[0m");
    assert_eq!(visible_width(&styled), visible_width(plain));
    assert_eq!(visible_width(&styled), plain.len());
}

#[test]
fn test_visible_width_ansi_cjk_still_correct() {
    let styled = "\x1b[31m预览标题\x1b[0m";
    assert_eq!(visible_width(styled), visible_width("预览标题"));
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

#[test]
fn extract_segments_plain_split_no_ansi() {
    let s = extract_segments("hello world", 3, 8, 3, false);
    assert_eq!(s.before, "hel");
    assert_eq!(s.before_width, 3);
    assert_eq!(s.after, "rld");
    assert_eq!(s.after_width, 3);
}

#[test]
fn extract_segments_after_inherits_before_sgr() {
    let line = "\x1b[1mabXYZde";
    let s = extract_segments(line, 2, 5, 2, false);
    assert_eq!(s.before, "\x1b[1mab");
    assert_eq!(s.before_width, 2);
    assert!(
        s.after.starts_with("\x1b[1m"),
        "after must inherit bold SGR, got: {:?}",
        s.after
    );
    assert!(
        s.after.contains("de"),
        "after must contain the trailing text"
    );
    assert_eq!(s.after_width, 2);
}

#[test]
fn extract_segments_after_len_zero_only_extracts_before() {
    let s = extract_segments("hello", 3, 0, 0, false);
    assert_eq!(s.before, "hel");
    assert_eq!(s.before_width, 3);
    assert_eq!(s.after, "");
    assert_eq!(s.after_width, 0);
}

#[test]
fn extract_segments_strict_after_rejects_wide_char_overflow() {
    let s = extract_segments("ab中cd", 2, 5, 1, true);
    assert_eq!(s.before, "ab");
    assert_eq!(s.before_width, 2);
    assert_eq!(s.after, "d");
    assert_eq!(s.after_width, 1);
}

#[test]
fn extract_segments_cjk_in_before() {
    let s = extract_segments("中xyz", 2, 2, 2, false);
    assert_eq!(s.before, "中");
    assert_eq!(s.before_width, 2);
    assert_eq!(s.after, "xy");
    assert_eq!(s.after_width, 2);
}
