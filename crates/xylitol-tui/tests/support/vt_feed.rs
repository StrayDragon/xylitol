use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use xylitol_tui::InputEvent;

fn key(code: KeyCode) -> InputEvent {
    InputEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn key_mod(code: KeyCode, mods: KeyModifiers) -> InputEvent {
    InputEvent::Key(KeyEvent::new(code, mods))
}

pub fn parse_vt_to_input_events(seq: &str) -> Vec<InputEvent> {
    let bytes = seq.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            let rest = &seq[i + 2..];
            if let Some(ev) = parse_csi(rest) {
                out.push(ev.0);
                i += 2 + ev.1;
                continue;
            }
        }
        if bytes[i] == 0x1b && i + 1 < bytes.len() {
            let next = bytes[i + 1];
            if next.is_ascii_alphanumeric() {
                out.push(key_mod(
                    KeyCode::Char(next.to_ascii_lowercase() as char),
                    KeyModifiers::ALT,
                ));
                i += 2;
                continue;
            }
        }
        if bytes[i] == 0x1b {
            out.push(key(KeyCode::Esc));
            i += 1;
            continue;
        }
        match bytes[i] {
            b'\r' | b'\n' => {
                out.push(key(KeyCode::Enter));
                i += 1;
            }
            b'\t' => {
                out.push(key(KeyCode::Tab));
                i += 1;
            }
            0x7f | 0x08 => {
                out.push(key(KeyCode::Backspace));
                i += 1;
            }
            c @ 0x01..=0x1a if c != b'\t' && c != b'\n' && c != b'\r' => {
                let ch = (c + b'a' - 1) as char;
                out.push(key_mod(KeyCode::Char(ch), KeyModifiers::CONTROL));
                i += 1;
            }
            0x1c => {
                out.push(key_mod(KeyCode::Char('\\'), KeyModifiers::CONTROL));
                i += 1;
            }
            0x1d => {
                out.push(key_mod(KeyCode::Char(']'), KeyModifiers::CONTROL));
                i += 1;
            }
            0x1f => {
                out.push(key_mod(KeyCode::Char('-'), KeyModifiers::CONTROL));
                i += 1;
            }
            0x00 => {
                out.push(key_mod(KeyCode::Char(' '), KeyModifiers::CONTROL));
                i += 1;
            }
            _ => {
                let s = &seq[i..];
                if let Some(ch) = s.chars().next() {
                    out.push(key(KeyCode::Char(ch)));
                    i += ch.len_utf8();
                } else {
                    i += 1;
                }
            }
        }
    }
    out
}

fn parse_csi(body: &str) -> Option<(InputEvent, usize)> {
    let bytes = body.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    match bytes[0] {
        b'A' => return Some((key(KeyCode::Up), 1)),
        b'B' => return Some((key(KeyCode::Down), 1)),
        b'C' => return Some((key(KeyCode::Right), 1)),
        b'D' => return Some((key(KeyCode::Left), 1)),
        b'H' => return Some((key(KeyCode::Home), 1)),
        b'F' => return Some((key(KeyCode::End), 1)),
        b'Z' => {
            return Some((key_mod(KeyCode::Tab, KeyModifiers::SHIFT), 1));
        }
        _ => {}
    }
    if let Some(tilde) = body.find('~') {
        let num: u32 = body[..tilde].parse().ok()?;
        let code = match num {
            2 => KeyCode::Insert,
            3 => KeyCode::Delete,
            5 => KeyCode::PageUp,
            6 => KeyCode::PageDown,
            15 => KeyCode::F(5),
            17 => KeyCode::F(6),
            18 => KeyCode::F(7),
            19 => KeyCode::F(8),
            20 => KeyCode::F(9),
            21 => KeyCode::F(10),
            23 => KeyCode::F(11),
            24 => KeyCode::F(12),
            _ => return None,
        };
        return Some((key(code), tilde + 1));
    }
    None
}

#[allow(dead_code)]
pub fn feed_vt(component: &mut dyn xylitol_tui::Component, seq: &str) {
    for ev in parse_vt_to_input_events(seq) {
        component.handle_input(ev);
    }
}
