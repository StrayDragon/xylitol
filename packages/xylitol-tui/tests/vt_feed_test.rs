//! Coverage for `tests/support/vt_feed.rs` VT → InputEvent translation.

mod support;

use crossterm::event::{KeyCode, KeyModifiers};
use support::vt_feed::parse_vt_to_input_events;
use xylitol_tui::InputEvent;

fn assert_key(ev: &InputEvent, code: KeyCode, mods: KeyModifiers) {
    match ev {
        InputEvent::Key(k) => {
            assert_eq!(k.code, code);
            assert_eq!(k.modifiers, mods);
        }
        InputEvent::Paste(_) => panic!("expected Key, got Paste"),
    }
}

#[test]
fn parses_printable_and_controls() {
    let evs = parse_vt_to_input_events("ab\r\x7f\x1b\t");
    assert_eq!(evs.len(), 6);
    assert_key(&evs[0], KeyCode::Char('a'), KeyModifiers::NONE);
    assert_key(&evs[1], KeyCode::Char('b'), KeyModifiers::NONE);
    assert_key(&evs[2], KeyCode::Enter, KeyModifiers::NONE);
    assert_key(&evs[3], KeyCode::Backspace, KeyModifiers::NONE);
    assert_key(&evs[4], KeyCode::Esc, KeyModifiers::NONE);
    assert_key(&evs[5], KeyCode::Tab, KeyModifiers::NONE);
}

#[test]
fn parses_csi_arrows_home_end_delete() {
    let evs = parse_vt_to_input_events("\x1b[A\x1b[B\x1b[C\x1b[D\x1b[H\x1b[F\x1b[3~");
    assert_eq!(evs.len(), 7);
    assert_key(&evs[0], KeyCode::Up, KeyModifiers::NONE);
    assert_key(&evs[1], KeyCode::Down, KeyModifiers::NONE);
    assert_key(&evs[2], KeyCode::Right, KeyModifiers::NONE);
    assert_key(&evs[3], KeyCode::Left, KeyModifiers::NONE);
    assert_key(&evs[4], KeyCode::Home, KeyModifiers::NONE);
    assert_key(&evs[5], KeyCode::End, KeyModifiers::NONE);
    assert_key(&evs[6], KeyCode::Delete, KeyModifiers::NONE);
}

#[test]
fn parses_ctrl_bytes_and_shift_tab() {
    let evs = parse_vt_to_input_events("\x10\x13\x03\x17\x1b[Z");
    assert_eq!(evs.len(), 5);
    assert_key(&evs[0], KeyCode::Char('p'), KeyModifiers::CONTROL);
    assert_key(&evs[1], KeyCode::Char('s'), KeyModifiers::CONTROL);
    assert_key(&evs[2], KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_key(&evs[3], KeyCode::Char('w'), KeyModifiers::CONTROL);
    assert_key(&evs[4], KeyCode::Tab, KeyModifiers::SHIFT);
}
