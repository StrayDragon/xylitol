//! Terminal color response parsing (OSC 11/10).
//!
//! Ported from pi's `terminal-colors.ts`. Parses background-color query replies
//! and `\e[?997;n` color-scheme reports from xterm-style terminals.
//!
//! Not yet wired into the event loop — the consumer (e.g. stdin_buffer or host
//! loop) should feed raw data to `is_osc_color_response` and then parse.

/// RGB color decoded from a terminal response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Terminal colour scheme preference reported by the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalColorScheme {
    Dark,
    Light,
}

// ── OSC 11 background-color response ────────────────────────────────────────
//
// Format (ITU T.416 / xterm):  \e]11;VALUE\a  or  \e]11;VALUE\e\\
// VALUE can be:
//   #RRGGBB          (6 hex digits)
//   #RRRRGGGGBBBB    (12 hex digits, high-depth)
//   rgb:RRRR/GGGG/BBBB

fn hex_to_rgb(hex: &str) -> Option<RgbColor> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(RgbColor { r, g, b })
}

fn parse_osc_hex_channel(ch: &str) -> Option<u8> {
    if !ch.chars().all(|c| c.is_ascii_hexdigit()) || ch.is_empty() {
        return None;
    }
    let max = (16u32.pow(ch.len() as u32)).saturating_sub(1);
    if max == 0 {
        return None;
    }
    let val = u32::from_str_radix(ch, 16).ok()?;
    Some(((val as f64 / max as f64) * 255.0).round() as u8)
}

/// Returns `true` if `data` looks like an OSC-11 background-color response.
pub fn is_osc11_background_color_response(data: &str) -> bool {
    // Match \e]11;...\a or \e]11;...\e\\
    data.starts_with("\x1b]11;") && (data.ends_with('\x07') || data.ends_with("\x1b\\"))
}

/// Parse an OSC-11 background-color response.
///
/// Returns `None` if the string isn't a valid OSC-11 response or its value
/// cannot be parsed.
pub fn parse_osc11_background_color(data: &str) -> Option<RgbColor> {
    // Strip the prefix \e]11; and the terminator
    let value = data
        .strip_prefix("\x1b]11;")?
        .trim_end_matches('\x07')
        .trim_end_matches("\x1b\\")
        .trim();

    if value.starts_with('#') {
        let hex = &value[1..];
        if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return hex_to_rgb(value);
        }
        if hex.len() == 12 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
            let r = parse_osc_hex_channel(&hex[0..4])?;
            let g = parse_osc_hex_channel(&hex[4..8])?;
            let b = parse_osc_hex_channel(&hex[8..12])?;
            return Some(RgbColor { r, g, b });
        }
        return None;
    }

    // rgb:RRRR/GGGG/BBBB (or rgba: variant)
    let rgb = value
        .strip_prefix("rgb:")
        .or_else(|| value.strip_prefix("rgba:"))?;
    let mut parts = rgb.splitn(3, '/');
    let r = parse_osc_hex_channel(parts.next()?)?;
    let g = parse_osc_hex_channel(parts.next()?)?;
    let b = parse_osc_hex_channel(parts.next()?)?;
    Some(RgbColor { r, g, b })
}

// ── color-scheme report ─────────────────────────────────────────────────────
//
// \e[?997;1n = dark
// \e[?997;2n = light

/// Parse a `\e[?997;n` color-scheme report. Returns `Some` if recognized.
pub fn parse_terminal_color_scheme_report(data: &str) -> Option<TerminalColorScheme> {
    let body = data.strip_prefix("\x1b[?997;")?;
    let code = body.strip_suffix('n')?;
    match code {
        "2" => Some(TerminalColorScheme::Light),
        _ => Some(TerminalColorScheme::Dark), // 1 or unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_rgb() {
        let c = parse_osc11_background_color("\x1b]11;#ff8800\x07").unwrap();
        assert_eq!(
            c,
            RgbColor {
                r: 0xff,
                g: 0x88,
                b: 0x00
            }
        );
    }

    #[test]
    fn parse_hex_12() {
        let c = parse_osc11_background_color("\x1b]11;#ffff00000000\x1b\\").unwrap();
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
    }

    #[test]
    fn parse_rgb_colons() {
        let c = parse_osc11_background_color("\x1b]11;rgb:ffff/0000/0000\x07").unwrap();
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
    }

    #[test]
    fn parse_rgba_colons() {
        let c = parse_osc11_background_color("\x1b]11;rgba:0000/ffff/0000\x07").unwrap();
        assert_eq!(c.r, 0);
        assert_eq!(c.g, 255);
        assert_eq!(c.b, 0);
    }

    #[test]
    fn reject_invalid() {
        assert!(parse_osc11_background_color("hello").is_none());
        assert!(parse_osc11_background_color("\x1b]11;#XYZ123\x07").is_none());
    }

    #[test]
    fn is_osc11_detection() {
        assert!(is_osc11_background_color_response("\x1b]11;#ffffff\x07"));
        assert!(is_osc11_background_color_response("\x1b]11;#ffffff\x1b\\"));
        assert!(!is_osc11_background_color_response("hello"));
    }

    #[test]
    fn color_scheme_detection() {
        assert!(matches!(
            parse_terminal_color_scheme_report("\x1b[?997;2n"),
            Some(TerminalColorScheme::Light)
        ));
        assert!(matches!(
            parse_terminal_color_scheme_report("\x1b[?997;1n"),
            Some(TerminalColorScheme::Dark)
        ));
        assert!(parse_terminal_color_scheme_report("hello").is_none());
    }
}
