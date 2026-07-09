use std::collections::HashMap;
use std::sync::Mutex;

static KITTY_PROTOCOL_ACTIVE: Mutex<bool> = Mutex::new(false);

pub fn set_kitty_protocol_active(active: bool) {
    *KITTY_PROTOCOL_ACTIVE.lock().unwrap() = active;
}

pub fn is_kitty_protocol_active() -> bool {
    *KITTY_PROTOCOL_ACTIVE.lock().unwrap()
}

pub type KeyId = &'static str;

const MOD_SHIFT: u32 = 1;
const MOD_ALT: u32 = 2;
const MOD_CTRL: u32 = 4;
const MOD_SUPER: u32 = 8;
const LOCK_MASK: u32 = 64 + 128;

const CODEPOINT_ESCAPE: u32 = 27;
const CODEPOINT_TAB: u32 = 9;
const CODEPOINT_ENTER: u32 = 13;
const CODEPOINT_SPACE: u32 = 32;
const CODEPOINT_BACKSPACE: u32 = 127;
const CODEPOINT_KP_ENTER: u32 = 57414;

const ARROW_UP: i32 = -1;
const ARROW_DOWN: i32 = -2;
const ARROW_RIGHT: i32 = -3;
const ARROW_LEFT: i32 = -4;

const FUNC_DELETE: i32 = -10;
const FUNC_INSERT: i32 = -11;
const FUNC_PAGEUP: i32 = -12;
const FUNC_PAGEDOWN: i32 = -13;
const FUNC_HOME: i32 = -14;
const FUNC_END: i32 = -15;

fn create_kitty_functional_map() -> HashMap<u32, i32> {
    let mut m = HashMap::new();
    m.insert(57399, 48);
    m.insert(57400, 49);
    m.insert(57401, 50);
    m.insert(57402, 51);
    m.insert(57403, 52);
    m.insert(57404, 53);
    m.insert(57405, 54);
    m.insert(57406, 55);
    m.insert(57407, 56);
    m.insert(57408, 57);
    m.insert(57409, 46);
    m.insert(57410, 47);
    m.insert(57411, 42);
    m.insert(57412, 45);
    m.insert(57413, 43);
    m.insert(57415, 61);
    m.insert(57416, 44);
    m.insert(57417, ARROW_LEFT);
    m.insert(57418, ARROW_RIGHT);
    m.insert(57419, ARROW_UP);
    m.insert(57420, ARROW_DOWN);
    m.insert(57421, FUNC_PAGEUP);
    m.insert(57422, FUNC_PAGEDOWN);
    m.insert(57423, FUNC_HOME);
    m.insert(57424, FUNC_END);
    m.insert(57425, FUNC_INSERT);
    m.insert(57426, FUNC_DELETE);
    m
}

fn get_kitty_func_cp(cp: u32) -> i32 {
    thread_local! {
        static MAP: HashMap<u32, i32> = create_kitty_functional_map();
    }
    MAP.with(|map| map.get(&cp).copied().unwrap_or(cp as i32))
}

fn normalize_shifted_letter_identity(cp: u32, modifier: u32) -> u32 {
    let effective = modifier & !LOCK_MASK;
    if (effective & MOD_SHIFT) != 0 && (65..=90).contains(&cp) {
        cp + 32
    } else {
        cp
    }
}

fn symbol_chars() -> &'static [char] {
    &[
        '`', '-', '=', '[', ']', '\\', ';', '\'', ',', '.', '/', '!', '@', '#', '$', '%', '^', '&',
        '*', '(', ')', '_', '+', '|', '~', '{', '}', ':', '<', '>', '?',
    ]
}

fn is_symbol_key(c: char) -> bool {
    symbol_chars().contains(&c)
}

// Kitty protocol parsing
#[derive(Debug, Clone)]
pub struct ParsedKittySequence {
    pub codepoint: u32,
    pub shifted_key: Option<u32>,
    pub base_layout_key: Option<u32>,
    pub modifier: u32,
    pub event_type: u8,
}

fn parse_event_type(s: Option<&str>) -> u8 {
    match s {
        Some("2") => 2,
        Some("3") => 3,
        _ => 1,
    }
}

pub fn parse_kitty_sequence(data: &str) -> Option<ParsedKittySequence> {
    let bytes = data.as_bytes();
    if bytes.len() < 3 || bytes[0] != 0x1b || bytes[1] != b'[' {
        return None;
    }

    let content = &data[2..];

    if let Some(inner) = content.strip_suffix('u') {
        return parse_kitty_csi_u(inner);
    } else if content.len() >= 4 && content.ends_with(['A', 'B', 'C', 'D']) {
        let direction = content.chars().last().unwrap();
        let arrow_codes: HashMap<char, i32> = [
            ('A', ARROW_UP),
            ('B', ARROW_DOWN),
            ('C', ARROW_RIGHT),
            ('D', ARROW_LEFT),
        ]
        .iter()
        .cloned()
        .collect();
        let codepoint = *arrow_codes.get(&direction)? as u32;
        let inner = &content[..content.len() - 1];
        if let Some(rest) = inner.strip_prefix("1;") {
            if let Some(colon_pos) = rest.find(':') {
                let mod_val: u32 = rest[..colon_pos].parse().unwrap_or(1);
                let event_str = &rest[colon_pos + 1..];
                let event_type = parse_event_type(Some(event_str));
                return Some(ParsedKittySequence {
                    codepoint,
                    shifted_key: None,
                    base_layout_key: None,
                    modifier: mod_val - 1,
                    event_type,
                });
            } else {
                let mod_val: u32 = rest.parse().unwrap_or(1);
                return Some(ParsedKittySequence {
                    codepoint,
                    shifted_key: None,
                    base_layout_key: None,
                    modifier: mod_val - 1,
                    event_type: 1,
                });
            }
        }
    } else if let Some(inner) = content.strip_suffix('~') {
        let parts: Vec<&str> = inner.split(';').collect();
        let key_num: u32 = parts[0].parse().ok()?;
        let func_codes: HashMap<u32, i32> = [
            (2, FUNC_INSERT),
            (3, FUNC_DELETE),
            (5, FUNC_PAGEUP),
            (6, FUNC_PAGEDOWN),
            (7, FUNC_HOME),
            (8, FUNC_END),
        ]
        .iter()
        .cloned()
        .collect();

        if let Some(&codepoint) = func_codes.get(&key_num) {
            let mod_val = if parts.len() > 1 {
                let mod_part = parts[1];
                if let Some(colon_pos) = mod_part.find(':') {
                    let m: u32 = mod_part[..colon_pos].parse().unwrap_or(1);
                    let event_str = &mod_part[colon_pos + 1..];
                    let event_type = parse_event_type(Some(event_str));
                    return Some(ParsedKittySequence {
                        codepoint: codepoint as u32,
                        shifted_key: None,
                        base_layout_key: None,
                        modifier: m - 1,
                        event_type,
                    });
                }
                parts[1].parse().unwrap_or(1)
            } else {
                1
            };
            return Some(ParsedKittySequence {
                codepoint: codepoint as u32,
                shifted_key: None,
                base_layout_key: None,
                modifier: mod_val - 1,
                event_type: 1,
            });
        }
    } else if content.len() >= 3 && (content.ends_with('H') || content.ends_with('F')) {
        let is_home = content.ends_with('H');
        let codepoint = if is_home {
            FUNC_HOME as u32
        } else {
            FUNC_END as u32
        };
        let inner = &content[..content.len() - 1];
        if let Some(rest) = inner.strip_prefix("1;") {
            if let Some(colon_pos) = rest.find(':') {
                let mod_val: u32 = rest[..colon_pos].parse().unwrap_or(1);
                let event_str = &rest[colon_pos + 1..];
                let event_type = parse_event_type(Some(event_str));
                return Some(ParsedKittySequence {
                    codepoint,
                    shifted_key: None,
                    base_layout_key: None,
                    modifier: mod_val - 1,
                    event_type,
                });
            } else {
                let mod_val: u32 = rest.parse().unwrap_or(1);
                return Some(ParsedKittySequence {
                    codepoint,
                    shifted_key: None,
                    base_layout_key: None,
                    modifier: mod_val - 1,
                    event_type: 1,
                });
            }
        }
    }
    None
}

fn parse_kitty_csi_u(inner: &str) -> Option<ParsedKittySequence> {
    // Two forms:
    // 1. <codepoint>u  (no modifier, plain key)
    // 2. <codepoint>[:<shifted>[:<base>]];<mod>[:<event>]u
    if let Some((cp_part, rest)) = inner.split_once(';') {
        let (mod_part, event_str) = if let Some(colon_pos) = rest.find(':') {
            (&rest[..colon_pos], Some(&rest[colon_pos + 1..]))
        } else {
            (rest, None)
        };
        let mod_val: u32 = mod_part.parse().unwrap_or(1);
        let event_type = parse_event_type(event_str);

        let cp_parts: Vec<&str> = cp_part.split(':').collect();
        let cp: u32 = cp_parts[0].parse().unwrap_or(0);
        let shifted_key = if cp_parts.len() > 1 && !cp_parts[1].is_empty() {
            cp_parts[1].parse().ok()
        } else {
            None
        };
        let base_layout_key = if cp_parts.len() > 2 {
            cp_parts[2].parse().ok()
        } else {
            None
        };

        Some(ParsedKittySequence {
            codepoint: cp,
            shifted_key,
            base_layout_key,
            modifier: mod_val - 1,
            event_type,
        })
    } else {
        // No semicolon: plain <codepoint>u
        let cp: u32 = inner.parse().unwrap_or(0);
        Some(ParsedKittySequence {
            codepoint: cp,
            shifted_key: None,
            base_layout_key: None,
            modifier: 0,
            event_type: 1,
        })
    }
}

fn matches_kitty_sequence(data: &str, expected_cp: u32, expected_mod: u32) -> bool {
    let parsed = match parse_kitty_sequence(data) {
        Some(p) => p,
        None => return false,
    };
    let actual_mod = parsed.modifier & !LOCK_MASK;
    let expected_mod = expected_mod & !LOCK_MASK;

    if actual_mod != expected_mod {
        return false;
    }

    let norm_cp = normalize_shifted_letter_identity(
        get_kitty_func_cp(parsed.codepoint) as u32,
        parsed.modifier,
    );
    let norm_expected =
        normalize_shifted_letter_identity(get_kitty_func_cp(expected_cp) as u32, expected_mod);

    if norm_cp == norm_expected {
        return true;
    }

    if let Some(base) = parsed.base_layout_key
        && base == expected_cp
    {
        let is_latin_letter = (97..=122).contains(&norm_cp);
        let is_known_symbol = char::from_u32(norm_cp).map(is_symbol_key).unwrap_or(false);
        if !is_latin_letter && !is_known_symbol {
            return true;
        }
    }

    false
}

fn parse_modify_other_keys(data: &str) -> Option<(u32, u32)> {
    if data.len() < 8 || !data.starts_with("\x1b[27;") || !data.ends_with('~') {
        return None;
    }
    let inner = &data[5..data.len() - 1];
    let parts: Vec<&str> = inner.split(';').collect();
    if parts.len() != 2 {
        return None;
    }
    let mod_val: u32 = parts[0].parse().ok()?;
    let cp: u32 = parts[1].parse().ok()?;
    Some((mod_val - 1, cp))
}

fn matches_modify_other_keys(data: &str, expected_cp: u32, expected_mod: u32) -> bool {
    if let Some((mod_val, cp)) = parse_modify_other_keys(data) {
        cp == expected_cp && mod_val == expected_mod
    } else {
        false
    }
}

fn matches_printable_modify_other_keys(data: &str, expected_cp: u32, expected_mod: u32) -> bool {
    if expected_mod == 0 {
        return false;
    }
    if let Some((mod_val, cp)) = parse_modify_other_keys(data) {
        if mod_val != expected_mod {
            return false;
        }
        normalize_shifted_letter_identity(cp, mod_val)
            == normalize_shifted_letter_identity(expected_cp, expected_mod)
    } else {
        false
    }
}

fn is_windows_terminal_session() -> bool {
    std::env::var("WT_SESSION").is_ok()
        && std::env::var("SSH_CONNECTION").is_err()
        && std::env::var("SSH_CLIENT").is_err()
        && std::env::var("SSH_TTY").is_err()
}

fn matches_raw_backspace(data: &str, expected_mod: u32) -> bool {
    if data == "\x7f" {
        return expected_mod == 0;
    }
    if data != "\x08" {
        return false;
    }
    if is_windows_terminal_session() {
        expected_mod == MOD_CTRL
    } else {
        expected_mod == 0
    }
}

fn raw_ctrl_char(key: char) -> Option<char> {
    let c = key.to_ascii_lowercase();
    let code = c as u32;
    if (97..=122).contains(&code) || c == '[' || c == '\\' || c == ']' || c == '_' {
        Some(char::from_u32(code & 0x1f).unwrap())
    } else if c == '-' {
        Some(char::from_u32(31).unwrap())
    } else {
        None
    }
}

struct ParsedKeyId {
    key: String,
    ctrl: bool,
    shift: bool,
    alt: bool,
    super_mod: bool,
}

fn parse_key_id(key_id: &str) -> Option<ParsedKeyId> {
    let lowered = key_id.to_lowercase();
    let parts: Vec<&str> = lowered.split('+').collect();
    let key = (*parts.last()?).to_string();
    Some(ParsedKeyId {
        key,
        ctrl: parts.contains(&"ctrl"),
        shift: parts.contains(&"shift"),
        alt: parts.contains(&"alt"),
        super_mod: parts.contains(&"super"),
    })
}

fn legacy_sequence_matches(data: &str, sequences: &[&str]) -> bool {
    sequences.contains(&data)
}

fn legacy_modifier_shift(data: &str, key: &str) -> bool {
    match key {
        "up" => data == "\x1b[a",
        "down" => data == "\x1b[b",
        "right" => data == "\x1b[c",
        "left" => data == "\x1b[d",
        "clear" => data == "\x1b[e",
        "insert" => data == "\x1b[2$",
        "delete" => data == "\x1b[3$",
        "pageUp" => data == "\x1b[5$",
        "pageDown" => data == "\x1b[6$",
        "home" => data == "\x1b[7$",
        "end" => data == "\x1b[8$",
        _ => false,
    }
}

fn legacy_modifier_ctrl(data: &str, key: &str) -> bool {
    match key {
        "up" => data == "\x1bOa",
        "down" => data == "\x1bOb",
        "right" => data == "\x1bOc",
        "left" => data == "\x1bOd",
        "clear" => data == "\x1bOe",
        "insert" => data == "\x1b[2^",
        "delete" => data == "\x1b[3^",
        "pageUp" => data == "\x1b[5^",
        "pageDown" => data == "\x1b[6^",
        "home" => data == "\x1b[7^",
        "end" => data == "\x1b[8^",
        _ => false,
    }
}

fn legacy_modifier_matches(data: &str, key: &str, modifier: u32) -> bool {
    if modifier == MOD_SHIFT {
        legacy_modifier_shift(data, key)
    } else if modifier == MOD_CTRL {
        legacy_modifier_ctrl(data, key)
    } else {
        false
    }
}

pub fn is_key_release(data: &str) -> bool {
    if data.contains("\x1b[200~") {
        return false;
    }
    data.contains(":3u")
        || data.contains(":3~")
        || data.contains(":3A")
        || data.contains(":3B")
        || data.contains(":3C")
        || data.contains(":3D")
        || data.contains(":3H")
        || data.contains(":3F")
}

pub fn is_key_repeat(data: &str) -> bool {
    if data.contains("\x1b[200~") {
        return false;
    }
    data.contains(":2u")
        || data.contains(":2~")
        || data.contains(":2A")
        || data.contains(":2B")
        || data.contains(":2C")
        || data.contains(":2D")
        || data.contains(":2H")
        || data.contains(":2F")
}

pub fn matches_key(data: &str, key_id: &str) -> bool {
    let parsed = match parse_key_id(key_id) {
        Some(p) => p,
        None => return false,
    };

    let mut modifier = 0u32;
    if parsed.shift {
        modifier |= MOD_SHIFT;
    }
    if parsed.alt {
        modifier |= MOD_ALT;
    }
    if parsed.ctrl {
        modifier |= MOD_CTRL;
    }
    if parsed.super_mod {
        modifier |= MOD_SUPER;
    }

    match parsed.key.as_str() {
        "escape" | "esc" => {
            if modifier != 0 {
                return false;
            }
            data == "\x1b"
                || matches_kitty_sequence(data, CODEPOINT_ESCAPE, 0)
                || matches_modify_other_keys(data, CODEPOINT_ESCAPE, 0)
        }
        "space" => {
            if !is_kitty_protocol_active() {
                if modifier == MOD_CTRL && data == "\x00" {
                    return true;
                }
                if modifier == MOD_ALT && data == "\x1b " {
                    return true;
                }
            }
            if modifier == 0 {
                data == " "
                    || matches_kitty_sequence(data, CODEPOINT_SPACE, 0)
                    || matches_modify_other_keys(data, CODEPOINT_SPACE, 0)
            } else {
                matches_kitty_sequence(data, CODEPOINT_SPACE, modifier)
                    || matches_modify_other_keys(data, CODEPOINT_SPACE, modifier)
            }
        }
        "tab" => {
            if modifier == MOD_SHIFT {
                data == "\x1b[Z"
                    || matches_kitty_sequence(data, CODEPOINT_TAB, MOD_SHIFT)
                    || matches_modify_other_keys(data, CODEPOINT_TAB, MOD_SHIFT)
            } else if modifier == 0 {
                data == "\t" || matches_kitty_sequence(data, CODEPOINT_TAB, 0)
            } else {
                matches_kitty_sequence(data, CODEPOINT_TAB, modifier)
                    || matches_modify_other_keys(data, CODEPOINT_TAB, modifier)
            }
        }
        "enter" | "return" => {
            if modifier == MOD_SHIFT {
                if matches_kitty_sequence(data, CODEPOINT_ENTER, MOD_SHIFT)
                    || matches_kitty_sequence(data, CODEPOINT_KP_ENTER, MOD_SHIFT)
                    || matches_modify_other_keys(data, CODEPOINT_ENTER, MOD_SHIFT)
                {
                    return true;
                }
                if is_kitty_protocol_active() {
                    return data == "\x1b\r" || data == "\n";
                }
                return false;
            }
            if modifier == MOD_ALT {
                if matches_kitty_sequence(data, CODEPOINT_ENTER, MOD_ALT)
                    || matches_kitty_sequence(data, CODEPOINT_KP_ENTER, MOD_ALT)
                    || matches_modify_other_keys(data, CODEPOINT_ENTER, MOD_ALT)
                {
                    return true;
                }
                if !is_kitty_protocol_active() {
                    return data == "\x1b\r";
                }
                return false;
            }
            if modifier == 0 {
                data == "\r"
                    || (!is_kitty_protocol_active() && data == "\n")
                    || data == "\x1bOM"
                    || matches_kitty_sequence(data, CODEPOINT_ENTER, 0)
                    || matches_kitty_sequence(data, CODEPOINT_KP_ENTER, 0)
            } else {
                matches_kitty_sequence(data, CODEPOINT_ENTER, modifier)
                    || matches_kitty_sequence(data, CODEPOINT_KP_ENTER, modifier)
                    || matches_modify_other_keys(data, CODEPOINT_ENTER, modifier)
            }
        }
        "backspace" => {
            if modifier == MOD_ALT {
                if data == "\x1b\x7f" || data == "\x1b\u{8}" {
                    return true;
                }
                return matches_kitty_sequence(data, CODEPOINT_BACKSPACE, MOD_ALT)
                    || matches_modify_other_keys(data, CODEPOINT_BACKSPACE, MOD_ALT);
            }
            if modifier == MOD_CTRL {
                if matches_raw_backspace(data, MOD_CTRL) {
                    return true;
                }
                return matches_kitty_sequence(data, CODEPOINT_BACKSPACE, MOD_CTRL)
                    || matches_modify_other_keys(data, CODEPOINT_BACKSPACE, MOD_CTRL);
            }
            if modifier == 0 {
                matches_raw_backspace(data, 0)
                    || matches_kitty_sequence(data, CODEPOINT_BACKSPACE, 0)
                    || matches_modify_other_keys(data, CODEPOINT_BACKSPACE, 0)
            } else {
                matches_kitty_sequence(data, CODEPOINT_BACKSPACE, modifier)
                    || matches_modify_other_keys(data, CODEPOINT_BACKSPACE, modifier)
            }
        }
        "insert" => {
            if modifier == 0 {
                legacy_sequence_matches(data, &["\x1b[2~"])
                    || matches_kitty_sequence(data, FUNC_INSERT as u32, 0)
            } else if legacy_modifier_matches(data, "insert", modifier) {
                true
            } else {
                matches_kitty_sequence(data, FUNC_INSERT as u32, modifier)
            }
        }
        "delete" => {
            if modifier == 0 {
                legacy_sequence_matches(data, &["\x1b[3~"])
                    || matches_kitty_sequence(data, FUNC_DELETE as u32, 0)
            } else if legacy_modifier_matches(data, "delete", modifier) {
                true
            } else {
                matches_kitty_sequence(data, FUNC_DELETE as u32, modifier)
            }
        }
        "home" => {
            if modifier == 0 {
                legacy_sequence_matches(data, &["\x1b[H", "\x1bOH", "\x1b[1~", "\x1b[7~"])
                    || matches_kitty_sequence(data, FUNC_HOME as u32, 0)
            } else if legacy_modifier_matches(data, "home", modifier) {
                true
            } else {
                matches_kitty_sequence(data, FUNC_HOME as u32, modifier)
            }
        }
        "end" => {
            if modifier == 0 {
                legacy_sequence_matches(data, &["\x1b[F", "\x1bOF", "\x1b[4~", "\x1b[8~"])
                    || matches_kitty_sequence(data, FUNC_END as u32, 0)
            } else if legacy_modifier_matches(data, "end", modifier) {
                true
            } else {
                matches_kitty_sequence(data, FUNC_END as u32, modifier)
            }
        }
        "pageup" => {
            if modifier == 0 {
                legacy_sequence_matches(data, &["\x1b[5~", "\x1b[[5~"])
                    || matches_kitty_sequence(data, FUNC_PAGEUP as u32, 0)
            } else if legacy_modifier_matches(data, "pageUp", modifier) {
                true
            } else {
                matches_kitty_sequence(data, FUNC_PAGEUP as u32, modifier)
            }
        }
        "pagedown" => {
            if modifier == 0 {
                legacy_sequence_matches(data, &["\x1b[6~", "\x1b[[6~"])
                    || matches_kitty_sequence(data, FUNC_PAGEDOWN as u32, 0)
            } else if legacy_modifier_matches(data, "pageDown", modifier) {
                true
            } else {
                matches_kitty_sequence(data, FUNC_PAGEDOWN as u32, modifier)
            }
        }
        "clear" => {
            if modifier == 0 {
                legacy_sequence_matches(data, &["\x1b[E", "\x1bOE"])
            } else {
                legacy_modifier_matches(data, "clear", modifier)
            }
        }
        "up" => {
            if modifier == MOD_ALT {
                return data == "\x1bp" || matches_kitty_sequence(data, ARROW_UP as u32, MOD_ALT);
            }
            if modifier == 0 {
                legacy_sequence_matches(data, &["\x1b[A", "\x1bOA"])
                    || matches_kitty_sequence(data, ARROW_UP as u32, 0)
            } else if legacy_modifier_matches(data, "up", modifier) {
                true
            } else {
                matches_kitty_sequence(data, ARROW_UP as u32, modifier)
            }
        }
        "down" => {
            if modifier == MOD_ALT {
                return data == "\x1bn" || matches_kitty_sequence(data, ARROW_DOWN as u32, MOD_ALT);
            }
            if modifier == 0 {
                legacy_sequence_matches(data, &["\x1b[B", "\x1bOB"])
                    || matches_kitty_sequence(data, ARROW_DOWN as u32, 0)
            } else if legacy_modifier_matches(data, "down", modifier) {
                true
            } else {
                matches_kitty_sequence(data, ARROW_DOWN as u32, modifier)
            }
        }
        "left" => {
            if modifier == MOD_ALT {
                return data == "\x1b[1;3D"
                    || (!is_kitty_protocol_active() && data == "\x1bB")
                    || data == "\x1bb"
                    || matches_kitty_sequence(data, ARROW_LEFT as u32, MOD_ALT);
            }
            if modifier == MOD_CTRL {
                return data == "\x1b[1;5D"
                    || legacy_modifier_matches(data, "left", MOD_CTRL)
                    || matches_kitty_sequence(data, ARROW_LEFT as u32, MOD_CTRL);
            }
            if modifier == 0 {
                legacy_sequence_matches(data, &["\x1b[D", "\x1bOD"])
                    || matches_kitty_sequence(data, ARROW_LEFT as u32, 0)
            } else if legacy_modifier_matches(data, "left", modifier) {
                true
            } else {
                matches_kitty_sequence(data, ARROW_LEFT as u32, modifier)
            }
        }
        "right" => {
            if modifier == MOD_ALT {
                return data == "\x1b[1;3C"
                    || (!is_kitty_protocol_active() && data == "\x1bF")
                    || data == "\x1bf"
                    || matches_kitty_sequence(data, ARROW_RIGHT as u32, MOD_ALT);
            }
            if modifier == MOD_CTRL {
                return data == "\x1b[1;5C"
                    || legacy_modifier_matches(data, "right", MOD_CTRL)
                    || matches_kitty_sequence(data, ARROW_RIGHT as u32, MOD_CTRL);
            }
            if modifier == 0 {
                legacy_sequence_matches(data, &["\x1b[C", "\x1bOC"])
                    || matches_kitty_sequence(data, ARROW_RIGHT as u32, 0)
            } else if legacy_modifier_matches(data, "right", modifier) {
                true
            } else {
                matches_kitty_sequence(data, ARROW_RIGHT as u32, modifier)
            }
        }
        "f1" => modifier == 0 && legacy_sequence_matches(data, &["\x1bOP", "\x1b[11~", "\x1b[[A"]),
        "f2" => modifier == 0 && legacy_sequence_matches(data, &["\x1bOQ", "\x1b[12~", "\x1b[[B"]),
        "f3" => modifier == 0 && legacy_sequence_matches(data, &["\x1bOR", "\x1b[13~", "\x1b[[C"]),
        "f4" => modifier == 0 && legacy_sequence_matches(data, &["\x1bOS", "\x1b[14~", "\x1b[[D"]),
        "f5" => modifier == 0 && legacy_sequence_matches(data, &["\x1b[15~", "\x1b[[E"]),
        "f6" => modifier == 0 && legacy_sequence_matches(data, &["\x1b[17~"]),
        "f7" => modifier == 0 && legacy_sequence_matches(data, &["\x1b[18~"]),
        "f8" => modifier == 0 && legacy_sequence_matches(data, &["\x1b[19~"]),
        "f9" => modifier == 0 && legacy_sequence_matches(data, &["\x1b[20~"]),
        "f10" => modifier == 0 && legacy_sequence_matches(data, &["\x1b[21~"]),
        "f11" => modifier == 0 && legacy_sequence_matches(data, &["\x1b[23~"]),
        "f12" => modifier == 0 && legacy_sequence_matches(data, &["\x1b[24~"]),
        _ => {
            if parsed.key.len() == 1 {
                let key_char = parsed.key.chars().next().unwrap();
                let cp = key_char as u32;
                let is_letter = key_char.is_ascii_lowercase();
                let is_digit = key_char.is_ascii_digit();

                let raw_ctrl = raw_ctrl_char(key_char);

                if modifier == MOD_CTRL + MOD_ALT
                    && !is_kitty_protocol_active()
                    && let Some(rc) = raw_ctrl
                    && data == format!("\x1b{}", rc)
                {
                    return true;
                }

                if modifier == MOD_ALT
                    && !is_kitty_protocol_active()
                    && (is_letter || is_digit)
                    && data == format!("\x1b{}", key_char)
                {
                    return true;
                }

                if modifier == MOD_CTRL {
                    if let Some(rc) = raw_ctrl
                        && data == rc.to_string()
                    {
                        return true;
                    }
                    return matches_kitty_sequence(data, cp, MOD_CTRL)
                        || matches_printable_modify_other_keys(data, cp, MOD_CTRL);
                }

                if modifier == MOD_SHIFT + MOD_CTRL {
                    return matches_kitty_sequence(data, cp, MOD_SHIFT + MOD_CTRL)
                        || matches_printable_modify_other_keys(data, cp, MOD_SHIFT + MOD_CTRL);
                }

                if modifier == MOD_SHIFT {
                    if is_letter && data == key_char.to_uppercase().to_string() {
                        return true;
                    }
                    return matches_kitty_sequence(data, cp, MOD_SHIFT)
                        || matches_printable_modify_other_keys(data, cp, MOD_SHIFT);
                }

                if modifier != 0 {
                    return matches_kitty_sequence(data, cp, modifier)
                        || matches_printable_modify_other_keys(data, cp, modifier);
                }

                return data == parsed.key || matches_kitty_sequence(data, cp, 0);
            }
            false
        }
    }
}

pub fn parse_key(data: &str) -> Option<String> {
    if let Some(kitty) = parse_kitty_sequence(data) {
        return format_parsed_key(
            get_kitty_func_cp(kitty.codepoint) as u32,
            kitty.modifier,
            kitty.base_layout_key,
        );
    }

    if let Some((mod_val, cp)) = parse_modify_other_keys(data) {
        return format_parsed_key(cp, mod_val, None);
    }

    let kitty_active = is_kitty_protocol_active();

    if kitty_active && (data == "\x1b\r" || data == "\n") {
        return Some("shift+enter".to_string());
    }

    match data {
        "\x1bOA" => return Some("up".to_string()),
        "\x1bOB" => return Some("down".to_string()),
        "\x1bOC" => return Some("right".to_string()),
        "\x1bOD" => return Some("left".to_string()),
        "\x1bOH" => return Some("home".to_string()),
        "\x1bOF" => return Some("end".to_string()),
        "\x1b[E" | "\x1bOE" => return Some("clear".to_string()),
        "\x1bOe" => return Some("ctrl+clear".to_string()),
        "\x1b[e" => return Some("shift+clear".to_string()),
        "\x1b[2~" => return Some("insert".to_string()),
        "\x1b[2$" => return Some("shift+insert".to_string()),
        "\x1b[2^" => return Some("ctrl+insert".to_string()),
        "\x1b[3$" => return Some("shift+delete".to_string()),
        "\x1b[3^" => return Some("ctrl+delete".to_string()),
        "\x1b[[5~" => return Some("pageUp".to_string()),
        "\x1b[[6~" => return Some("pageDown".to_string()),
        "\x1b[a" => return Some("shift+up".to_string()),
        "\x1b[b" => return Some("shift+down".to_string()),
        "\x1b[c" => return Some("shift+right".to_string()),
        "\x1b[d" => return Some("shift+left".to_string()),
        "\x1bOa" => return Some("ctrl+up".to_string()),
        "\x1bOb" => return Some("ctrl+down".to_string()),
        "\x1bOc" => return Some("ctrl+right".to_string()),
        "\x1bOd" => return Some("ctrl+left".to_string()),
        "\x1b[5$" => return Some("shift+pageUp".to_string()),
        "\x1b[6$" => return Some("shift+pageDown".to_string()),
        "\x1b[7$" => return Some("shift+home".to_string()),
        "\x1b[8$" => return Some("shift+end".to_string()),
        "\x1b[5^" => return Some("ctrl+pageUp".to_string()),
        "\x1b[6^" => return Some("ctrl+pageDown".to_string()),
        "\x1b[7^" => return Some("ctrl+home".to_string()),
        "\x1b[8^" => return Some("ctrl+end".to_string()),
        "\x1bOP" | "\x1b[11~" | "\x1b[[A" => return Some("f1".to_string()),
        "\x1bOQ" | "\x1b[12~" | "\x1b[[B" => return Some("f2".to_string()),
        "\x1bOR" | "\x1b[13~" | "\x1b[[C" => return Some("f3".to_string()),
        "\x1bOS" | "\x1b[14~" | "\x1b[[D" => return Some("f4".to_string()),
        "\x1b[[E" | "\x1b[15~" => return Some("f5".to_string()),
        "\x1b[17~" => return Some("f6".to_string()),
        "\x1b[18~" => return Some("f7".to_string()),
        "\x1b[19~" => return Some("f8".to_string()),
        "\x1b[20~" => return Some("f9".to_string()),
        "\x1b[21~" => return Some("f10".to_string()),
        "\x1b[23~" => return Some("f11".to_string()),
        "\x1b[24~" => return Some("f12".to_string()),
        "\x1bb" => return Some("alt+left".to_string()),
        "\x1bf" => return Some("alt+right".to_string()),
        "\x1bp" => return Some("alt+up".to_string()),
        "\x1bn" => return Some("alt+down".to_string()),
        _ => {}
    }

    if data == "\x1b" {
        return Some("escape".to_string());
    }
    if data == "\x1c" {
        return Some("ctrl+\\".to_string());
    }
    if data == "\x1d" {
        return Some("ctrl+]".to_string());
    }
    if data == "\x1f" {
        return Some("ctrl+-".to_string());
    }
    if data == "\x1b\x1b" {
        return Some("ctrl+alt+[".to_string());
    }
    if data == "\x1b\x1c" {
        return Some("ctrl+alt+\\".to_string());
    }
    if data == "\x1b\x1d" {
        return Some("ctrl+alt+]".to_string());
    }
    if data == "\x1b\x1f" {
        return Some("ctrl+alt+-".to_string());
    }
    if data == "\t" {
        return Some("tab".to_string());
    }
    if data == "\r" || (!kitty_active && data == "\n") || data == "\x1bOM" {
        return Some("enter".to_string());
    }
    if data == "\x00" {
        return Some("ctrl+space".to_string());
    }
    if data == " " {
        return Some("space".to_string());
    }
    if data == "\x7f" {
        return Some("backspace".to_string());
    }
    if data == "\x08" {
        return if is_windows_terminal_session() {
            Some("ctrl+backspace".to_string())
        } else {
            Some("backspace".to_string())
        };
    }
    if data == "\x1b[Z" {
        return Some("shift+tab".to_string());
    }
    if !kitty_active && data == "\x1b\r" {
        return Some("alt+enter".to_string());
    }
    if !kitty_active && data == "\x1b " {
        return Some("alt+space".to_string());
    }
    if data == "\x1b\x7f" || data == "\x1b\u{8}" {
        return Some("alt+backspace".to_string());
    }
    if !kitty_active && data == "\x1bB" {
        return Some("alt+left".to_string());
    }
    if !kitty_active && data == "\x1bF" {
        return Some("alt+right".to_string());
    }

    if !kitty_active && data.len() == 2 && data.starts_with('\x1b') {
        let code = data.as_bytes()[1];
        if (1..=26).contains(&code) {
            return Some(format!("ctrl+alt+{}", (code + 96) as char));
        }
        if (97..=122).contains(&code) || (48..=57).contains(&code) {
            return Some(format!("alt+{}", code as char));
        }
    }

    if data == "\x1b[A" {
        return Some("up".to_string());
    }
    if data == "\x1b[B" {
        return Some("down".to_string());
    }
    if data == "\x1b[C" {
        return Some("right".to_string());
    }
    if data == "\x1b[D" {
        return Some("left".to_string());
    }
    if data == "\x1b[H" || data == "\x1bOH" {
        return Some("home".to_string());
    }
    if data == "\x1b[F" || data == "\x1bOF" {
        return Some("end".to_string());
    }
    if data == "\x1b[3~" {
        return Some("delete".to_string());
    }
    if data == "\x1b[5~" {
        return Some("pageUp".to_string());
    }
    if data == "\x1b[6~" {
        return Some("pageDown".to_string());
    }

    if data.len() == 1 {
        let code = data.as_bytes()[0];
        if (1..=26).contains(&code) {
            return Some(format!("ctrl+{}", (code + 96) as char));
        }
        if (32..=126).contains(&code) {
            return Some(data.to_string());
        }
    }

    None
}

fn format_parsed_key(cp: u32, modifier: u32, base_layout_key: Option<u32>) -> Option<String> {
    let norm_cp = get_kitty_func_cp(cp) as u32;
    let identity_cp = normalize_shifted_letter_identity(norm_cp, modifier);

    let is_latin_letter = (97..=122).contains(&identity_cp);
    let is_digit = (48..=57).contains(&identity_cp);
    let is_known_symbol = char::from_u32(identity_cp)
        .map(is_symbol_key)
        .unwrap_or(false);

    let effective_cp = if is_latin_letter || is_digit || is_known_symbol {
        identity_cp
    } else {
        base_layout_key.unwrap_or(identity_cp)
    };

    let key_name: &str = match effective_cp as i32 {
        x if x == CODEPOINT_ESCAPE as i32 => "escape",
        x if x == CODEPOINT_TAB as i32 => "tab",
        x if x == CODEPOINT_ENTER as i32 || x == CODEPOINT_KP_ENTER as i32 => "enter",
        x if x == CODEPOINT_SPACE as i32 => "space",
        x if x == CODEPOINT_BACKSPACE as i32 => "backspace",
        x if x == FUNC_DELETE => "delete",
        x if x == FUNC_INSERT => "insert",
        x if x == FUNC_HOME => "home",
        x if x == FUNC_END => "end",
        x if x == FUNC_PAGEUP => "pageUp",
        x if x == FUNC_PAGEDOWN => "pageDown",
        x if x == ARROW_UP => "up",
        x if x == ARROW_DOWN => "down",
        x if x == ARROW_LEFT => "left",
        x if x == ARROW_RIGHT => "right",
        _ => {
            if (48..=57).contains(&effective_cp) {
                return format_key_name_with_modifiers(
                    &String::from(char::from_u32(effective_cp).unwrap()),
                    modifier,
                );
            }
            if (97..=122).contains(&effective_cp) {
                return format_key_name_with_modifiers(
                    &String::from(char::from_u32(effective_cp).unwrap()),
                    modifier,
                );
            }
            if let Some(c) = char::from_u32(effective_cp)
                && is_symbol_key(c)
            {
                return format_key_name_with_modifiers(&c.to_string(), modifier);
            }
            return None;
        }
    };

    format_key_name_with_modifiers(key_name, modifier)
}

fn format_key_name_with_modifiers(key_name: &str, modifier: u32) -> Option<String> {
    let mut mods: Vec<&str> = Vec::new();
    let effective = modifier & !LOCK_MASK;
    let supported_mask = MOD_SHIFT | MOD_CTRL | MOD_ALT | MOD_SUPER;
    if (effective & !supported_mask) != 0 {
        return None;
    }
    if effective & MOD_SHIFT != 0 {
        mods.push("shift");
    }
    if effective & MOD_CTRL != 0 {
        mods.push("ctrl");
    }
    if effective & MOD_ALT != 0 {
        mods.push("alt");
    }
    if effective & MOD_SUPER != 0 {
        mods.push("super");
    }

    if mods.is_empty() {
        Some(key_name.to_string())
    } else {
        mods.push(key_name);
        Some(mods.join("+"))
    }
}

pub fn decode_kitty_printable(data: &str) -> Option<String> {
    if !data.starts_with("\x1b[") || !data.ends_with('u') {
        return None;
    }
    let inner = &data[2..data.len() - 1];

    // Check for form without semicolon: plain <codepoint>u
    if !inner.contains(';') {
        let cp: u32 = inner.parse().ok()?;
        if cp == 0 || cp < 32 {
            return None;
        }
        let effective = get_kitty_func_cp(cp) as u32;
        return char::from_u32(effective).map(|c| c.to_string());
    }

    let semicolon = inner.find(';')?;
    let cp_part = &inner[..semicolon];
    let rest = &inner[semicolon + 1..];

    let cp_str = cp_part.split(':').next()?;
    let cp: u32 = cp_str.parse().ok()?;
    if cp == 0 {
        return None;
    }

    let shifted_key: Option<u32> = {
        let parts: Vec<&str> = cp_part.split(':').collect();
        if parts.len() > 1 && !parts[1].is_empty() {
            parts[1].parse().ok()
        } else {
            None
        }
    };

    let mod_str = rest.split(':').next()?;
    let mod_val: u32 = mod_str.parse().unwrap_or(1);
    let modifier = if mod_val > 0 { mod_val - 1 } else { 0 };

    let allowed = MOD_SHIFT | LOCK_MASK;
    if (modifier & !allowed) != 0 {
        return None;
    }
    if modifier & (MOD_ALT | MOD_CTRL) != 0 {
        return None;
    }

    let effective_cp = if modifier & MOD_SHIFT != 0 {
        shifted_key.unwrap_or(cp)
    } else {
        cp
    };
    let effective_cp = get_kitty_func_cp(effective_cp) as u32;
    if effective_cp < 32 {
        return None;
    }

    char::from_u32(effective_cp).map(|c| c.to_string())
}

fn decode_modify_other_keys_printable(data: &str) -> Option<String> {
    let (modifier, cp) = parse_modify_other_keys(data)?;
    let effective = modifier & !LOCK_MASK;
    if (effective & !MOD_SHIFT) != 0 {
        return None;
    }
    if cp < 32 {
        return None;
    }
    char::from_u32(cp).map(|c| c.to_string())
}

pub fn decode_printable_key(data: &str) -> Option<String> {
    decode_kitty_printable(data).or_else(|| decode_modify_other_keys_printable(data))
}

/// Match a crossterm `KeyEvent` against a key id (`"ctrl+c"`, `"up"`, …).
///
/// Only `Press` and `Repeat` match. Compares `KeyCode` + `KeyModifiers` via
/// [`parse_key_id`] — the runtime path; VT `matches_key` remains for unit tests.
pub fn matches_key_event(event: &crossterm::event::KeyEvent, key_id: &str) -> bool {
    use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};

    if !matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return false;
    }

    let parsed = match parse_key_id(key_id) {
        Some(p) => p,
        None => return false,
    };

    let has_ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
    let has_alt = event.modifiers.contains(KeyModifiers::ALT);
    let has_shift = event.modifiers.contains(KeyModifiers::SHIFT);
    let has_super = event.modifiers.contains(KeyModifiers::SUPER);

    if has_ctrl != parsed.ctrl
        || has_alt != parsed.alt
        || has_shift != parsed.shift
        || has_super != parsed.super_mod
    {
        return false;
    }

    match parsed.key.as_str() {
        "escape" | "esc" => matches!(event.code, KeyCode::Esc),
        "enter" | "return" => matches!(event.code, KeyCode::Enter),
        "tab" => matches!(event.code, KeyCode::Tab),
        "backspace" => matches!(event.code, KeyCode::Backspace),
        "delete" => matches!(event.code, KeyCode::Delete),
        "insert" => matches!(event.code, KeyCode::Insert),
        "home" => matches!(event.code, KeyCode::Home),
        "end" => matches!(event.code, KeyCode::End),
        "pageup" => matches!(event.code, KeyCode::PageUp),
        "pagedown" => matches!(event.code, KeyCode::PageDown),
        "up" => matches!(event.code, KeyCode::Up),
        "down" => matches!(event.code, KeyCode::Down),
        "left" => matches!(event.code, KeyCode::Left),
        "right" => matches!(event.code, KeyCode::Right),
        "space" => matches!(event.code, KeyCode::Char(' ')),
        "clear" => matches!(event.code, KeyCode::Null), // unused at runtime
        k if k.starts_with('f') => {
            if let Ok(n) = k[1..].parse::<u8>() {
                matches!(event.code, KeyCode::F(m) if m == n)
            } else {
                false
            }
        }
        k if k.len() == 1 => {
            let expected = k.chars().next().unwrap();
            match event.code {
                KeyCode::Char(c) if expected.is_ascii_alphabetic() => {
                    c.eq_ignore_ascii_case(&expected)
                }
                KeyCode::Char(c) => c == expected,
                _ => false,
            }
        }
        _ => false,
    }
}

/// Printable text from a key event: `KeyCode::Char` with no ctrl/alt/super.
/// Shift is allowed (uppercase / shifted symbols).
pub fn printable_from_key_event(event: &crossterm::event::KeyEvent) -> Option<String> {
    use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};

    if !matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return None;
    }
    if event
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
    {
        return None;
    }
    match event.code {
        KeyCode::Char(c) => Some(c.to_string()),
        _ => None,
    }
}
