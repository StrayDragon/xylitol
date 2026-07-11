use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthChar;

/// Background painter closure: maps a plain-text line to an ANSI-styled line.
/// Shared by Panel/Text components that apply an optional per-line background.
pub type BgFn = Box<dyn Fn(&str) -> String>;

/// Calculate the terminal width of a single grapheme cluster.
pub(crate) fn grapheme_width(g: &str) -> usize {
    if g == "\t" {
        return 3;
    }
    {
        let mut width = 0usize;
        for c in g.chars() {
            width += c.width().unwrap_or(0);
        }
        width
    }
}

/// Calculate the visible width of a string in terminal columns (ANSI codes stripped).
pub fn visible_width(s: &str) -> usize {
    if s.is_empty() {
        return 0;
    }

    // Fast path: pure ASCII printable
    if s.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
        return s.len();
    }

    // Strip ANSI escape sequences
    let clean = strip_ansi_codes(s);

    // Measure grapheme clusters
    let mut width = 0usize;
    for g in UnicodeSegmentation::graphemes(clean.as_str(), true) {
        width += grapheme_width(g);
    }
    width
}

/// Strip ANSI escape sequences from a string.
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() {
            match bytes[i + 1] {
                b'[' => {
                    // CSI sequence: ESC [ ... m/G/K/H/J
                    let mut j = i + 2;
                    while j < bytes.len() && ![b'm', b'G', b'K', b'H', b'J'].contains(&bytes[j]) {
                        j += 1;
                    }
                    if j < bytes.len() {
                        i = j + 1;
                        continue;
                    }
                }
                b']' => {
                    // OSC sequence: ESC ] ... BEL or ESC ] ... ST
                    let mut j = i + 2;
                    while j < bytes.len() {
                        if bytes[j] == 0x07 {
                            i = j + 1;
                            break;
                        }
                        if bytes[j] == 0x1b && j + 1 < bytes.len() && bytes[j + 1] == b'\\' {
                            i = j + 2;
                            break;
                        }
                        j += 1;
                    }
                    if j >= bytes.len() {
                        i = bytes.len();
                    }
                    continue;
                }
                b'_' => {
                    // APC sequence
                    let mut j = i + 2;
                    while j < bytes.len() {
                        if bytes[j] == 0x07 {
                            i = j + 1;
                            break;
                        }
                        if bytes[j] == 0x1b && j + 1 < bytes.len() && bytes[j + 1] == b'\\' {
                            i = j + 2;
                            break;
                        }
                        j += 1;
                    }
                    if j >= bytes.len() {
                        i = bytes.len();
                    }
                    continue;
                }
                _ => {}
            }
        }
        // Copy one UTF-8 scalar — never `bytes[i] as char` (splits `·` → Â·, +1 width).
        let ch = s[i..].chars().next().expect("i in bounds");
        result.push(ch);
        i += ch.len_utf8();
    }
    result
}

/// Extract an ANSI escape code at the given byte position, returning the code and its length.
pub fn extract_ansi_code(s: &str, pos: usize) -> Option<(&str, usize)> {
    let bytes = s.as_bytes();
    if pos >= bytes.len() || bytes[pos] != 0x1b {
        return None;
    }

    let next = bytes.get(pos + 1)?;

    match next {
        b'[' => {
            let mut j = pos + 2;
            while j < bytes.len() && ![b'm', b'G', b'K', b'H', b'J'].contains(&bytes[j]) {
                j += 1;
            }
            if j < bytes.len() {
                Some((&s[pos..=j], j + 1 - pos))
            } else {
                None
            }
        }
        b']' => {
            let mut j = pos + 2;
            while j < bytes.len() {
                if bytes[j] == 0x07 {
                    return Some((&s[pos..=j], j + 1 - pos));
                }
                if bytes[j] == 0x1b && j + 1 < bytes.len() && bytes[j + 1] == b'\\' {
                    return Some((&s[pos..=j + 1], j + 2 - pos));
                }
                j += 1;
            }
            None
        }
        b'_' => {
            let mut j = pos + 2;
            while j < bytes.len() {
                if bytes[j] == 0x07 {
                    return Some((&s[pos..=j], j + 1 - pos));
                }
                if bytes[j] == 0x1b && j + 1 < bytes.len() && bytes[j + 1] == b'\\' {
                    return Some((&s[pos..=j + 1], j + 2 - pos));
                }
                j += 1;
            }
            None
        }
        _ => None,
    }
}

/// Track active ANSI SGR codes to preserve styling across line breaks.
#[derive(Default)]
struct AnsiCodeTracker {
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    blink: bool,
    inverse: bool,
    hidden: bool,
    strikethrough: bool,
    fg_color: Option<String>,
    bg_color: Option<String>,
    active_hyperlink: Option<ActiveHyperlink>,
}

struct ActiveHyperlink {
    params: String,
    url: String,
    terminator: String,
}

impl AnsiCodeTracker {
    fn new() -> Self {
        Self {
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            blink: false,
            inverse: false,
            hidden: false,
            strikethrough: false,
            fg_color: None,
            bg_color: None,
            active_hyperlink: None,
        }
    }

    fn process(&mut self, code: &str) {
        // OSC 8 hyperlink
        if code.starts_with("\x1b]8;") {
            let terminator = if code.ends_with('\x07') {
                "\x07"
            } else {
                "\x1b\\"
            };
            let tlen = terminator.len();
            let body = &code[4..code.len() - tlen];
            if let Some(sep) = body.find(';') {
                let params = body[..sep].to_string();
                let url = body[sep + 1..].to_string();
                if url.is_empty() {
                    self.active_hyperlink = None;
                } else {
                    self.active_hyperlink = Some(ActiveHyperlink {
                        params,
                        url,
                        terminator: terminator.to_string(),
                    });
                }
            }
            return;
        }

        if !code.ends_with('m') {
            return;
        }

        // Parse SGR parameters
        let params_str = &code[2..code.len() - 1]; // strip \x1b[ and m
        if params_str.is_empty() || params_str == "0" {
            self.reset();
            return;
        }

        let parts: Vec<&str> = params_str.split(';').collect();
        let mut i = 0;
        while i < parts.len() {
            let code_val: i32 = parts[i].parse().unwrap_or(-1);
            match code_val {
                0 => self.reset(),
                1 => self.bold = true,
                2 => self.dim = true,
                3 => self.italic = true,
                4 => self.underline = true,
                5 => self.blink = true,
                7 => self.inverse = true,
                8 => self.hidden = true,
                9 => self.strikethrough = true,
                21 => self.bold = false,
                22 => {
                    self.bold = false;
                    self.dim = false;
                }
                23 => self.italic = false,
                24 => self.underline = false,
                25 => self.blink = false,
                27 => self.inverse = false,
                28 => self.hidden = false,
                29 => self.strikethrough = false,
                39 => self.fg_color = None,
                49 => self.bg_color = None,
                38 | 48 => {
                    // 256-color or RGB
                    if i + 2 < parts.len() && parts[i + 1] == "5" {
                        let color = format!("{};5;{}", code_val, parts[i + 2]);
                        if code_val == 38 {
                            self.fg_color = Some(color);
                        } else {
                            self.bg_color = Some(color);
                        }
                        i += 2;
                    } else if i + 4 < parts.len() && parts[i + 1] == "2" {
                        let color = format!(
                            "{};2;{};{};{}",
                            code_val,
                            parts[i + 2],
                            parts[i + 3],
                            parts[i + 4]
                        );
                        if code_val == 38 {
                            self.fg_color = Some(color);
                        } else {
                            self.bg_color = Some(color);
                        }
                        i += 4;
                    }
                }
                _ => {
                    // Standard foreground 30-37, 90-97
                    if (30..=37).contains(&code_val) || (90..=97).contains(&code_val) {
                        self.fg_color = Some(code_val.to_string());
                    } else if (40..=47).contains(&code_val) || (100..=107).contains(&code_val) {
                        self.bg_color = Some(code_val.to_string());
                    }
                }
            }
            i += 1;
        }
    }

    fn reset(&mut self) {
        self.bold = false;
        self.dim = false;
        self.italic = false;
        self.underline = false;
        self.blink = false;
        self.inverse = false;
        self.hidden = false;
        self.strikethrough = false;
        self.fg_color = None;
        self.bg_color = None;
        // SGR reset does not affect OSC 8 hyperlink state
    }

    /// Full reset including hyperlink state.
    /// 预留：对齐 pi `AnsiCodeTracker.clear`（extract_segments 完整移植时启用）。
    #[allow(dead_code)]
    fn clear(&mut self) {
        self.reset();
        self.active_hyperlink = None;
    }

    pub(crate) fn get_active_codes(&self) -> String {
        let mut codes: Vec<String> = Vec::new();
        if self.bold {
            codes.push("1".to_string());
        }
        if self.dim {
            codes.push("2".to_string());
        }
        if self.italic {
            codes.push("3".to_string());
        }
        if self.underline {
            codes.push("4".to_string());
        }
        if self.blink {
            codes.push("5".to_string());
        }
        if self.inverse {
            codes.push("7".to_string());
        }
        if self.hidden {
            codes.push("8".to_string());
        }
        if self.strikethrough {
            codes.push("9".to_string());
        }
        if let Some(ref c) = self.fg_color {
            codes.push(c.clone());
        }
        if let Some(ref c) = self.bg_color {
            codes.push(c.clone());
        }

        let mut result = if codes.is_empty() {
            String::new()
        } else {
            format!("\x1b[{}m", codes.join(";"))
        };

        if let Some(ref hl) = self.active_hyperlink {
            result.push_str(&format!("\x1b]8;{};{}{}", hl.params, hl.url, hl.terminator));
        }
        result
    }

    /// Whether any SGR attribute is active.
    /// 预留：对齐 pi `hasActiveCodes`（extract_segments 完整移植时启用）。
    #[allow(dead_code)]
    fn has_active_codes(&self) -> bool {
        self.bold
            || self.dim
            || self.italic
            || self.underline
            || self.blink
            || self.inverse
            || self.hidden
            || self.strikethrough
            || self.fg_color.is_some()
            || self.bg_color.is_some()
            || self.active_hyperlink.is_some()
    }

    fn get_line_end_reset(&self) -> String {
        let mut result = String::new();
        if self.underline {
            result.push_str("\x1b[24m");
        }
        if let Some(ref hl) = self.active_hyperlink {
            result.push_str(&format!("\x1b]8;;{}", hl.terminator));
        }
        result
    }
}

/// Result of splitting a line into the region before an overlay and the region
/// after it, used by the engine's overlay compositing.
pub struct ExtractedSegments {
    pub before: String,
    pub before_width: usize,
    pub after: String,
    pub after_width: usize,
}

/// Split `line` into two column-bounded regions: `before` covers `[0, beforeEnd)`
/// and `after` covers `[afterStart, afterStart + afterLen)`. The `after` region
/// inherits the SGR styling accumulated across `before` (so an overlay laid over
/// the middle of a line doesn't leave the trailing content unstyled), and the
/// first grapheme of `after` is preceded by the active SGR codes at that point.
///
/// Direct port of pi-tui's `extractSegments` (utils.ts:1117). `strict_after`
/// rejects graphemes that would cross `afterEnd` (wide-char safety); when
/// `afterLen == 0`, only `before` is extracted and we stop at `beforeEnd`.
pub fn extract_segments(
    line: &str,
    before_end: usize,
    after_start: usize,
    after_len: usize,
    strict_after: bool,
) -> ExtractedSegments {
    let mut before = String::new();
    let mut before_width = 0usize;
    let mut after = String::new();
    let mut after_width = 0usize;

    let mut current_col = 0usize;
    let mut pending_ansi_before = String::new();
    let mut after_started = false;
    let after_end = after_start + after_len;
    let stop_col = if after_len == 0 {
        before_end
    } else {
        after_end
    };

    // Fresh tracker so the inherited styling reflects only this line's SGR.
    let mut tracker = AnsiCodeTracker::default();

    let mut i = 0;
    while i < line.len() {
        if let Some((code, len)) = extract_ansi_code(&line[i..], 0) {
            // Track all SGR codes so we know styling at afterStart.
            tracker.process(code);
            // Route the raw code into whichever segment current_col falls in.
            if current_col < before_end {
                pending_ansi_before.push_str(code);
            } else if current_col >= after_start && current_col < after_end && after_started {
                // Only include ANSI in `after` once styling has been prepended;
                // otherwise it would precede the inherited SGR blob.
                after.push_str(code);
            }
            i += len;
            continue;
        }

        // Consume the run of non-ANSI text up to the next escape, advancing by
        // whole chars so multi-byte sequences (CJK/emoji) never split mid-char.
        let mut text_end = i;
        while text_end < line.len() {
            // An ESC at text_end starts an ANSI sequence; stop here.
            if line.as_bytes()[text_end] == 0x1b {
                break;
            }
            // Advance one whole char (UTF-8 boundary safe).
            match line[text_end..].chars().next() {
                Some(c) => text_end += c.len_utf8(),
                None => break,
            }
        }
        let chunk = &line[i..text_end];
        for grapheme in chunk.graphemes(true) {
            let w = grapheme_width(grapheme);

            if current_col < before_end && current_col + w <= before_end {
                if !pending_ansi_before.is_empty() {
                    before.push_str(&std::mem::take(&mut pending_ansi_before));
                }
                before.push_str(grapheme);
                before_width += w;
            } else if current_col >= after_start && current_col < after_end {
                let fits = !strict_after || current_col + w <= after_end;
                if fits {
                    if !after_started {
                        // Prepend inherited styling from before the overlay.
                        after.push_str(&tracker.get_active_codes());
                        after_started = true;
                    }
                    after.push_str(grapheme);
                    after_width += w;
                }
            }

            current_col += w;
            if current_col >= stop_col {
                break;
            }
        }
        i = text_end;
        if current_col >= stop_col {
            break;
        }
    }

    ExtractedSegments {
        before,
        before_width,
        after,
        after_width,
    }
}

fn update_tracker_from_text(text: &str, tracker: &mut AnsiCodeTracker) {
    let mut i = 0;
    while i < text.len() {
        if let Some((code, len)) = extract_ansi_code(&text[i..], 0) {
            tracker.process(code);
            i += len;
        } else {
            // Move forward one char
            if let Some(c) = text[i..].chars().next() {
                i += c.len_utf8();
            } else {
                i += 1;
            }
        }
    }
}

/// Split text into tokens while keeping ANSI codes attached.
fn split_into_tokens_with_ansi(text: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut pending_ansi = String::new();
    let mut current_kind: Option<TokenKind> = None;
    let mut i = 0;

    while i < text.len() {
        if let Some((code, len)) = extract_ansi_code(&text[i..], 0) {
            pending_ansi.push_str(code);
            i += len;
            continue;
        }

        // Find end of non-ANSI segment
        let mut end = i;
        while end < text.len() {
            if extract_ansi_code(&text[end..], 0).is_some() {
                break;
            }
            end += text[end..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
        }

        let segment = &text[i..end];
        for g in UnicodeSegmentation::graphemes(segment, true) {
            let is_space = g == " ";
            if !is_space
                && g.chars().any(|c| {
                    // CJK break regex equivalent
                    ('\u{2E80}'..='\u{2EFF}').contains(&c)
                        || ('\u{3000}'..='\u{303F}').contains(&c)
                        || ('\u{31C0}'..='\u{31EF}').contains(&c)
                        || ('\u{3200}'..='\u{32FF}').contains(&c)
                        || ('\u{3300}'..='\u{33FF}').contains(&c)
                        || ('\u{3400}'..='\u{4DBF}').contains(&c)
                        || ('\u{4E00}'..='\u{9FFF}').contains(&c)
                        || ('\u{F900}'..='\u{FAFF}').contains(&c)
                        || ('\u{FE30}'..='\u{FE4F}').contains(&c)
                        || ('\u{3040}'..='\u{309F}').contains(&c)
                        || ('\u{30A0}'..='\u{30FF}').contains(&c)
                        || ('\u{AC00}'..='\u{D7AF}').contains(&c)
                        || ('\u{1100}'..='\u{11FF}').contains(&c)
                        || ('\u{3130}'..='\u{318F}').contains(&c)
                        || ('\u{A960}'..='\u{A97F}').contains(&c)
                        || ('\u{D7B0}'..='\u{D7FF}').contains(&c)
                        || ('\u{3100}'..='\u{312F}').contains(&c)
                })
            {
                // CJK character: flush current and emit as standalone token
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
                let token = format!("{}{}", pending_ansi, g);
                pending_ansi.clear();
                tokens.push(token);
                current_kind = None;
                continue;
            }

            let kind = if is_space {
                TokenKind::Space
            } else {
                TokenKind::Word
            };
            if !current.is_empty() && current_kind != Some(kind) {
                tokens.push(std::mem::take(&mut current));
            }

            if !pending_ansi.is_empty() {
                current.push_str(&pending_ansi);
                pending_ansi.clear();
            }

            current_kind = Some(kind);
            current.push_str(g);
        }

        i = end;
    }

    if !pending_ansi.is_empty() {
        if !current.is_empty() {
            current.push_str(&pending_ansi);
        } else if let Some(last) = tokens.last_mut() {
            last.push_str(&pending_ansi);
        } else {
            current = pending_ansi;
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Space,
    Word,
}

/// Wrap text with ANSI codes preserved. Returns lines where each line is <= `width` visible chars.
pub fn wrap_text_with_ansi(text: &str, max_width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let input_lines: Vec<&str> = text.split('\n').collect();
    let mut result: Vec<String> = Vec::new();
    let mut tracker = AnsiCodeTracker::new();

    for input_line in input_lines {
        let prefix = if !result.is_empty() {
            tracker.get_active_codes()
        } else {
            String::new()
        };
        let wrapped = wrap_single_line(&format!("{}{}", prefix, input_line), max_width);
        for line in wrapped {
            result.push(line);
        }
        update_tracker_from_text(input_line, &mut tracker);
    }

    if result.is_empty() {
        vec![String::new()]
    } else {
        result
    }
}

fn wrap_single_line(line: &str, width: usize) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }

    let visible_len = visible_width(line);
    if visible_len <= width {
        return vec![line.to_string()];
    }

    let mut wrapped: Vec<String> = Vec::new();
    let mut tracker = AnsiCodeTracker::new();
    let tokens = split_into_tokens_with_ansi(line);

    let mut current_line = String::new();
    let mut current_visible = 0usize;

    for token in tokens {
        let token_visible = visible_width(&token);
        let is_whitespace = token.trim().is_empty();

        // Token too long - break character by character
        if token_visible > width && !is_whitespace {
            if !current_line.is_empty() {
                let line_end_reset = tracker.get_line_end_reset();
                if !line_end_reset.is_empty() {
                    current_line.push_str(&line_end_reset);
                }
                wrapped.push(std::mem::take(&mut current_line));
            }

            let broken = break_long_word(&token, width, &mut tracker);
            for line in &broken[..broken.len() - 1] {
                wrapped.push(line.clone());
            }
            current_line = broken.last().cloned().unwrap_or_default();
            current_visible = visible_width(&current_line);
            continue;
        }

        let total_needed = current_visible + token_visible;
        if total_needed > width && current_visible > 0 {
            let mut line_to_wrap = current_line.trim_end().to_string();
            let line_end_reset = tracker.get_line_end_reset();
            if !line_end_reset.is_empty() {
                line_to_wrap.push_str(&line_end_reset);
            }
            wrapped.push(line_to_wrap);
            if is_whitespace {
                current_line = tracker.get_active_codes();
                current_visible = 0;
            } else {
                current_line = format!("{}{}", tracker.get_active_codes(), token);
                current_visible = token_visible;
            }
        } else {
            current_line.push_str(&token);
            current_visible += token_visible;
        }

        update_tracker_from_text(&token, &mut tracker);
    }

    if !current_line.is_empty() {
        wrapped.push(current_line);
    }

    if wrapped.is_empty() {
        vec![String::new()]
    } else {
        wrapped
            .into_iter()
            .map(|line| line.trim_end().to_string())
            .collect()
    }
}

fn break_long_word(word: &str, width: usize, tracker: &mut AnsiCodeTracker) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current_line = tracker.get_active_codes();
    let mut current_width = 0usize;

    // Separate ANSI codes from graphemes
    let mut i = 0;
    let mut segments: Vec<Segment> = Vec::new();

    while i < word.len() {
        if let Some((code, len)) = extract_ansi_code(&word[i..], 0) {
            segments.push(Segment::Ansi(code.to_string()));
            i += len;
        } else {
            // Find end of non-ANSI segment
            let mut end = i;
            while end < word.len() && extract_ansi_code(&word[end..], 0).is_none() {
                end += word[end..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(1);
            }
            for g in UnicodeSegmentation::graphemes(&word[i..end], true) {
                segments.push(Segment::Grapheme(g.to_string()));
            }
            i = end;
        }
    }

    for seg in segments {
        match seg {
            Segment::Ansi(code) => {
                current_line.push_str(&code);
                tracker.process(&code);
            }
            Segment::Grapheme(g) => {
                if g.is_empty() {
                    continue;
                }
                let gw = grapheme_width(&g);

                if current_width + gw > width {
                    let line_end_reset = tracker.get_line_end_reset();
                    if !line_end_reset.is_empty() {
                        current_line.push_str(&line_end_reset);
                    }
                    lines.push(std::mem::take(&mut current_line));
                    current_line = tracker.get_active_codes();
                    current_width = 0;
                }

                current_line.push_str(&g);
                current_width += gw;
            }
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    }
}

enum Segment {
    Ansi(String),
    Grapheme(String),
}

/// Truncate text to fit within a maximum visible width.
pub fn truncate_to_width(text: &str, max_width: usize, ellipsis: &str, pad: bool) -> String {
    if max_width == 0 {
        return String::new();
    }
    if text.is_empty() {
        return if pad {
            " ".repeat(max_width)
        } else {
            String::new()
        };
    }

    let ellipsis_width = visible_width(ellipsis);
    if ellipsis_width >= max_width {
        let text_width = visible_width(text);
        if text_width <= max_width {
            return if pad {
                format!("{}{}", text, " ".repeat(max_width - text_width))
            } else {
                text.to_string()
            };
        }
        let clipped = truncate_fragment_to_width(ellipsis, max_width);
        return finalize_truncated("", 0, &clipped.0, clipped.1, max_width, pad);
    }

    // Fast path: pure ASCII
    if text.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
        if text.len() <= max_width {
            return if pad {
                format!("{}{}", text, " ".repeat(max_width - text.len()))
            } else {
                text.to_string()
            };
        }
        let target = max_width - ellipsis_width;
        let prefix = &text[..target];
        return finalize_truncated(prefix, target, ellipsis, ellipsis_width, max_width, pad);
    }

    let target = max_width - ellipsis_width;
    let mut result = String::new();
    let mut pending_ansi = String::new();
    let mut kept_width = 0usize;
    let mut keep_prefix = true;
    let mut visible_so_far = 0usize;
    let mut overflowed = false;

    let has_ansi = text.contains('\x1b');

    if !has_ansi {
        for g in UnicodeSegmentation::graphemes(text, true) {
            let w = grapheme_width(g);
            if keep_prefix && kept_width + w <= target {
                result.push_str(g);
                kept_width += w;
            } else {
                keep_prefix = false;
            }
            visible_so_far += w;
            if visible_so_far > max_width {
                overflowed = true;
                break;
            }
        }
    } else {
        let mut i = 0;
        while i < text.len() {
            if let Some((code, len)) = extract_ansi_code(&text[i..], 0) {
                pending_ansi.push_str(code);
                i += len;
                continue;
            }

            let mut end = i;
            while end < text.len() && extract_ansi_code(&text[end..], 0).is_none() {
                end += text[end..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(1);
            }

            for g in UnicodeSegmentation::graphemes(&text[i..end], true) {
                let w = grapheme_width(g);
                if keep_prefix && kept_width + w <= target {
                    if !pending_ansi.is_empty() {
                        result.push_str(&pending_ansi);
                        pending_ansi.clear();
                    }
                    result.push_str(g);
                    kept_width += w;
                } else {
                    keep_prefix = false;
                    pending_ansi.clear();
                }
                visible_so_far += w;
                if visible_so_far > max_width {
                    overflowed = true;
                    break;
                }
            }
            if overflowed {
                break;
            }
            i = end;
        }
    }

    if !overflowed {
        return if pad {
            format!(
                "{}{}",
                text,
                " ".repeat(max_width.saturating_sub(visible_so_far))
            )
        } else {
            text.to_string()
        };
    }

    finalize_truncated(
        &result,
        kept_width,
        ellipsis,
        ellipsis_width,
        max_width,
        pad,
    )
}

fn truncate_fragment_to_width(text: &str, max_width: usize) -> (String, usize) {
    if max_width == 0 || text.is_empty() {
        return (String::new(), 0);
    }
    if text.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
        let clipped = &text[..std::cmp::min(text.len(), max_width)];
        return (clipped.to_string(), clipped.len());
    }

    let mut result = String::new();
    let mut width = 0usize;
    for g in UnicodeSegmentation::graphemes(text, true) {
        let w = grapheme_width(g);
        if width + w > max_width {
            break;
        }
        result.push_str(g);
        width += w;
    }
    (result, width)
}

fn finalize_truncated(
    prefix: &str,
    prefix_width: usize,
    ellipsis: &str,
    ellipsis_width: usize,
    max_width: usize,
    pad: bool,
) -> String {
    let reset = "\x1b[0m";
    let visible = prefix_width + ellipsis_width;
    let mut result = format!("{}{}{}{}", prefix, reset, ellipsis, reset);

    if pad && visible < max_width {
        result.push_str(&" ".repeat(max_width - visible));
    }
    result
}

/// Extract a slice of text by visible column range.
pub fn slice_by_column(line: &str, start_col: usize, length: usize) -> String {
    slice_with_width(line, start_col, length, false).0
}

/// Like `slice_by_column` but rejects graphemes that would cross the end column
/// (strict mode). Use this when a width-bound region must not split a wide
/// char (e.g. an input field's visible window) — mirrors pi's strict=true calls.
pub fn slice_by_column_strict(line: &str, start_col: usize, length: usize) -> String {
    slice_with_width(line, start_col, length, true).0
}

pub(crate) fn slice_with_width(
    line: &str,
    start_col: usize,
    length: usize,
    strict: bool,
) -> (String, usize) {
    if length == 0 {
        return (String::new(), 0);
    }
    let end_col = start_col + length;
    let mut result = String::new();
    let mut result_width = 0usize;
    let mut current_col = 0usize;
    let mut i = 0;
    let mut pending_ansi = String::new();

    while i < line.len() {
        if let Some((code, len)) = extract_ansi_code(&line[i..], 0) {
            if current_col >= start_col && current_col < end_col {
                result.push_str(code);
            } else if current_col < start_col {
                pending_ansi.push_str(code);
            }
            i += len;
            continue;
        }

        let mut end = i;
        while end < line.len() && extract_ansi_code(&line[end..], 0).is_none() {
            end += line[end..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
        }

        for g in UnicodeSegmentation::graphemes(&line[i..end], true) {
            let w = grapheme_width(g);
            let in_range = current_col >= start_col && current_col < end_col;
            let fits = !strict || current_col + w <= end_col;
            if in_range && fits {
                if !pending_ansi.is_empty() {
                    result.push_str(&pending_ansi);
                    pending_ansi.clear();
                }
                result.push_str(g);
                result_width += w;
            }
            current_col += w;
            if current_col >= end_col {
                break;
            }
        }
        i = end;
        if current_col >= end_col {
            break;
        }
    }

    (result, result_width)
}

/// Check if a character is whitespace.
pub fn is_whitespace_char(c: char) -> bool {
    c.is_whitespace()
}

/// Check if a character is punctuation.
pub fn is_punctuation_char(c: char) -> bool {
    c.is_ascii_punctuation()
}

/// Apply background color to a line, padding to full width.
pub fn apply_background_to_line(
    line: &str,
    width: usize,
    bg_fn: &dyn Fn(&str) -> String,
) -> String {
    let visible_len = visible_width(line);
    let padding = width.saturating_sub(visible_len);
    let with_padding = format!("{}{}", line, " ".repeat(padding));
    bg_fn(&with_padding)
}

/// Whether to keep the head or tail when truncating to a visual-line budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TruncateFrom {
    #[default]
    Tail,
    Head,
}

/// Result of [`truncate_to_visual_lines`] (pi `truncateToVisualLines`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualTruncateResult {
    pub visual_lines: Vec<String>,
    pub skipped_count: usize,
}

/// Wrap-aware truncate to at most `max_visual_lines` (ANSI-safe via [`wrap_text_with_ansi`]).
///
/// `Tail` keeps the last N lines (bash/tool live preview). `Head` keeps the first N.
/// When `max_visual_lines` is `usize::MAX`, returns all wrapped lines with `skipped_count = 0`.
pub fn truncate_to_visual_lines(
    text: &str,
    max_visual_lines: usize,
    width: usize,
    from: TruncateFrom,
) -> VisualTruncateResult {
    if text.is_empty() {
        return VisualTruncateResult {
            visual_lines: Vec::new(),
            skipped_count: 0,
        };
    }
    let all = wrap_text_with_ansi(&text.replace('\t', "   "), width.max(1));
    if max_visual_lines == usize::MAX || all.len() <= max_visual_lines {
        return VisualTruncateResult {
            visual_lines: all,
            skipped_count: 0,
        };
    }
    let skipped = all.len() - max_visual_lines;
    let visual_lines = match from {
        TruncateFrom::Tail => all[skipped..].to_vec(),
        TruncateFrom::Head => all[..max_visual_lines].to_vec(),
    };
    VisualTruncateResult {
        visual_lines,
        skipped_count: skipped,
    }
}

#[cfg(test)]
mod visible_width_tests {
    use super::*;

    #[test]
    fn middle_dot_is_one_column_with_or_without_ansi() {
        // Regression: strip_ansi used `bytes[i] as char`, splitting U+00B7 into Â· (+1 width).
        let plain = "cwd · model";
        assert_eq!(visible_width(plain), 11);
        let styled = format!("\x1b[31m{plain}\x1b[0m");
        assert_eq!(visible_width(&styled), 11);
    }
}
