pub const OSC11_BG_QUERY: &str = "\x1b]11;?\x07";

pub const CSI_COLOR_SCHEME_QUERY: &str = "\x1b[?996n";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalColorScheme {
    Dark,
    Light,
}

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

pub fn is_osc11_background_color_response(data: &str) -> bool {
    data.starts_with("\x1b]11;") && (data.ends_with('\x07') || data.ends_with("\x1b\\"))
}

pub fn parse_osc11_background_color(data: &str) -> Option<RgbColor> {
    let value = data
        .strip_prefix("\x1b]11;")?
        .trim_end_matches('\x07')
        .trim_end_matches("\x1b\\")
        .trim();

    if value.starts_with('#') {
        let hex = value.strip_prefix('#').unwrap_or("");
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

    let rgb = value
        .strip_prefix("rgb:")
        .or_else(|| value.strip_prefix("rgba:"))?;
    let mut parts = rgb.splitn(3, '/');
    let r = parse_osc_hex_channel(parts.next()?)?;
    let g = parse_osc_hex_channel(parts.next()?)?;
    let b = parse_osc_hex_channel(parts.next()?)?;
    Some(RgbColor { r, g, b })
}

pub fn parse_terminal_color_scheme_report(data: &str) -> Option<TerminalColorScheme> {
    let body = data.strip_prefix("\x1b[?997;")?;
    let code = body.strip_suffix('n')?;
    match code {
        "2" => Some(TerminalColorScheme::Light),
        _ => Some(TerminalColorScheme::Dark),
    }
}

pub fn relative_luminance(rgb: RgbColor) -> f64 {
    (0.299 * f64::from(rgb.r) + 0.587 * f64::from(rgb.g) + 0.114 * f64::from(rgb.b)) / 255.0
}

pub fn scheme_from_background_rgb(rgb: RgbColor) -> TerminalColorScheme {
    if relative_luminance(rgb) >= 0.5 {
        TerminalColorScheme::Light
    } else {
        TerminalColorScheme::Dark
    }
}

pub fn parse_colorfgbg(value: &str) -> Option<TerminalColorScheme> {
    let bg = value.split(';').nth(1)?.trim().parse::<u8>().ok()?;
    Some(if bg >= 7 {
        TerminalColorScheme::Light
    } else {
        TerminalColorScheme::Dark
    })
}

#[derive(Debug, Clone, Default)]
pub struct ThemeDetectSources<'a> {
    pub explicit: Option<TerminalColorScheme>,
    pub osc11_background: Option<RgbColor>,
    pub color_scheme_report: Option<TerminalColorScheme>,
    pub colorfgbg: Option<&'a str>,
}

pub fn resolve_terminal_color_scheme(sources: ThemeDetectSources<'_>) -> TerminalColorScheme {
    if let Some(s) = sources.explicit {
        return s;
    }
    if let Some(rgb) = sources.osc11_background {
        return scheme_from_background_rgb(rgb);
    }
    if let Some(s) = sources.color_scheme_report {
        return s;
    }
    if let Some(raw) = sources.colorfgbg
        && let Some(s) = parse_colorfgbg(raw)
    {
        return s;
    }
    TerminalColorScheme::Dark
}

pub fn is_terminal_color_reply(data: &str) -> bool {
    is_osc11_background_color_response(data) || parse_terminal_color_scheme_report(data).is_some()
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

    #[test]
    fn luminance_and_scheme_from_rgb() {
        assert!(
            relative_luminance(RgbColor {
                r: 255,
                g: 255,
                b: 255
            }) >= 0.5
        );
        assert!(
            relative_luminance(RgbColor {
                r: 30,
                g: 30,
                b: 46
            }) < 0.5
        );
        assert_eq!(
            scheme_from_background_rgb(RgbColor {
                r: 239,
                g: 241,
                b: 245
            }),
            TerminalColorScheme::Light
        );
        assert_eq!(
            scheme_from_background_rgb(RgbColor {
                r: 30,
                g: 30,
                b: 46
            }),
            TerminalColorScheme::Dark
        );
    }

    #[test]
    fn colorfgbg_parse() {
        assert_eq!(parse_colorfgbg("0;15"), Some(TerminalColorScheme::Light));
        assert_eq!(parse_colorfgbg("15;0"), Some(TerminalColorScheme::Dark));
        assert!(parse_colorfgbg("broken").is_none());
    }

    #[test]
    fn resolve_priority_osc11_over_env() {
        let scheme = resolve_terminal_color_scheme(ThemeDetectSources {
            explicit: None,
            osc11_background: Some(RgbColor {
                r: 255,
                g: 255,
                b: 255,
            }),
            color_scheme_report: None,
            colorfgbg: Some("15;0"),
        });
        assert_eq!(scheme, TerminalColorScheme::Light);
    }

    #[test]
    fn query_constants_and_reply_detect() {
        assert!(OSC11_BG_QUERY.contains("]11;?"));
        assert!(CSI_COLOR_SCHEME_QUERY.contains("996"));
        assert!(is_terminal_color_reply("\x1b]11;#ffffff\x07"));
        assert!(is_terminal_color_reply("\x1b[?997;2n"));
        assert!(!is_terminal_color_reply("hello"));
    }
}
