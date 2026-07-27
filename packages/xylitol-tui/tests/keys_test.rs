use xylitol_tui::keys::*;

#[test]
fn test_legacy_ctrl_c() {
    with_kitty_protocol_active(false, || {
        assert!(matches_key("\x03", "ctrl+c"));
    });
}

#[test]
fn test_legacy_ctrl_d() {
    with_kitty_protocol_active(false, || {
        assert!(matches_key("\x04", "ctrl+d"));
    });
}

#[test]
fn test_escape_key() {
    with_kitty_protocol_active(false, || {
        assert!(matches_key("\x1b", "escape"));
    });
}

#[test]
fn test_legacy_enter() {
    with_kitty_protocol_active(false, || {
        assert!(matches_key("\n", "enter"));
        assert_eq!(parse_key("\n"), Some("enter".to_string()));
    });
}

#[test]
fn test_shift_enter_when_kitty_active() {
    with_kitty_protocol_active(true, || {
        assert!(matches_key("\n", "shift+enter"));
        assert!(!matches_key("\n", "enter"));
        assert_eq!(parse_key("\n"), Some("shift+enter".to_string()));
    });
}

#[test]
fn test_ctrl_space() {
    with_kitty_protocol_active(false, || {
        assert!(matches_key("\x00", "ctrl+space"));
        assert_eq!(parse_key("\x00"), Some("ctrl+space".to_string()));
    });
}

#[test]
fn test_ctrl_symbols() {
    with_kitty_protocol_active(false, || {
        assert!(matches_key("\x1c", "ctrl+\\"));
        assert_eq!(parse_key("\x1c"), Some("ctrl+\\".to_string()));
        assert!(matches_key("\x1d", "ctrl+]"));
        assert_eq!(parse_key("\x1d"), Some("ctrl+]".to_string()));
        assert!(matches_key("\x1f", "ctrl+-"));
        assert_eq!(parse_key("\x1f"), Some("ctrl+-".to_string()));
    });
}

#[test]
fn test_arrow_keys() {
    with_kitty_protocol_active(false, || {
        assert!(matches_key("\x1b[A", "up"));
        assert!(matches_key("\x1b[B", "down"));
        assert!(matches_key("\x1b[C", "right"));
        assert!(matches_key("\x1b[D", "left"));
    });
}

#[test]
fn test_ss3_arrows() {
    with_kitty_protocol_active(false, || {
        assert!(matches_key("\x1bOA", "up"));
        assert!(matches_key("\x1bOB", "down"));
        assert!(matches_key("\x1bOC", "right"));
        assert!(matches_key("\x1bOD", "left"));
        assert!(matches_key("\x1bOH", "home"));
        assert!(matches_key("\x1bOF", "end"));
    });
}

#[test]
fn test_function_keys() {
    with_kitty_protocol_active(false, || {
        assert!(matches_key("\x1bOP", "f1"));
        assert!(matches_key("\x1b[24~", "f12"));
        assert!(matches_key("\x1b[E", "clear"));
    });
}

#[test]
fn test_alt_arrows() {
    with_kitty_protocol_active(false, || {
        assert!(matches_key("\x1bp", "alt+up"));
        assert!(!matches_key("\x1bp", "up"));
    });
}

#[test]
fn test_rxvt_modifiers() {
    with_kitty_protocol_active(false, || {
        assert!(matches_key("\x1b[a", "shift+up"));
        assert!(matches_key("\x1bOa", "ctrl+up"));
        assert!(matches_key("\x1b[2$", "shift+insert"));
        assert!(matches_key("\x1b[2^", "ctrl+insert"));
        assert!(matches_key("\x1b[7$", "shift+home"));
    });
}

#[test]
fn test_parse_special_keys() {
    with_kitty_protocol_active(false, || {
        assert_eq!(parse_key("\x1b"), Some("escape".to_string()));
        assert_eq!(parse_key("\t"), Some("tab".to_string()));
        assert_eq!(parse_key("\r"), Some("enter".to_string()));
        assert_eq!(parse_key(" "), Some("space".to_string()));
        assert_eq!(parse_key("1"), Some("1".to_string()));
    });
}

#[test]
fn test_parse_arrows() {
    with_kitty_protocol_active(false, || {
        assert_eq!(parse_key("\x1b[A"), Some("up".to_string()));
        assert_eq!(parse_key("\x1b[B"), Some("down".to_string()));
        assert_eq!(parse_key("\x1b[C"), Some("right".to_string()));
        assert_eq!(parse_key("\x1b[D"), Some("left".to_string()));
    });
}

#[test]
fn test_kitty_ctrl_c() {
    with_kitty_protocol_active(true, || {
        assert!(matches_key("\x1b[99;5u", "ctrl+c"));
        assert_eq!(parse_key("\x1b[99;5u"), Some("ctrl+c".to_string()));
    });
}

#[test]
fn test_kitty_super_modifier() {
    with_kitty_protocol_active(true, || {
        assert!(matches_key("\x1b[107;9u", "super+k"));
        assert_eq!(parse_key("\x1b[107;9u"), Some("super+k".to_string()));
    });
}

#[test]
fn test_kitty_non_latin_layout() {
    with_kitty_protocol_active(true, || {
        // Cyrillic 'с' = 1089, base key 'c' = 99
        assert!(matches_key("\x1b[1089::99;5u", "ctrl+c"));
    });
}

#[test]
fn test_kitty_ctrl_shift_p() {
    with_kitty_protocol_active(true, || {
        assert!(matches_key("\x1b[1079::112;6u", "ctrl+shift+p"));
    });
}

#[test]
fn test_modify_other_keys() {
    with_kitty_protocol_active(false, || {
        assert!(matches_key("\x1b[27;5;99~", "ctrl+c"));
        assert_eq!(parse_key("\x1b[27;5;99~"), Some("ctrl+c".to_string()));
        assert!(matches_key("\x1b[27;2;13~", "shift+enter"));
        assert!(matches_key("\x1b[27;2;9~", "shift+tab"));
    });
}

#[test]
fn matches_key_event_shift_tab_accepts_back_tab() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    with_kitty_protocol_active(false, || {
        assert!(matches_key_event(
            &KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT),
            "shift+tab"
        ));
        assert!(matches_key_event(
            &KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE),
            "shift+tab"
        ));
        assert!(!matches_key_event(
            &KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE),
            "tab"
        ));
        assert!(!matches_key_event(
            &KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
            "shift+tab"
        ));
    });
}

#[test]
fn test_is_key_release() {
    with_kitty_protocol_active(true, || {
        assert!(is_key_release("\x1b[97:3u"));
        assert!(is_key_repeat("\x1b[97:2u"));
    });
}

#[test]
fn test_debug_not_release_with_paste() {
    with_kitty_protocol_active(false, || {
        assert!(!is_key_release("\x1b[200~90:62:3F:A5\x1b[201~"));
    });
}

#[test]
fn test_decode_kitty_printable() {
    with_kitty_protocol_active(false, || {
        assert_eq!(decode_kitty_printable("\x1b[97u"), Some("a".to_string()));
        assert_eq!(decode_kitty_printable("\x1b[49u"), Some("1".to_string()));
        // KP keys
        assert_eq!(decode_kitty_printable("\x1b[57400u"), Some("1".to_string()));
        // Navigation keys should not decode
        assert_eq!(decode_kitty_printable("\x1b[57417u"), None);
    });
}

#[test]
fn test_kitty_functional_keys() {
    with_kitty_protocol_active(true, || {
        assert!(matches_key("\x1b[57410u", "/"));
        assert!(matches_key("\x1b[57417u", "left"));
        assert!(matches_key("\x1b[57426u", "delete"));
        assert_eq!(parse_key("\x1b[57399u"), Some("0".to_string()));
        assert_eq!(parse_key("\x1b[57413u"), Some("+".to_string()));
    });
}

#[test]
fn test_backspace() {
    with_kitty_protocol_active(false, || {
        assert!(matches_key("\x7f", "backspace"));
        assert!(matches_key("\x08", "backspace"));
        // \x08 matches ctrl+h
        assert!(matches_key("\x08", "ctrl+h"));
    });
}
