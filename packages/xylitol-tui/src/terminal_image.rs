//! Terminal image protocol support (Kitty, iTerm2) + capability detection.
//!
//! Ported from pi's `terminal-image.ts`.

use std::sync::Mutex;

// ── types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageProtocol {
    Kitty,
    ITerm2,
}

#[derive(Debug, Clone, Copy)]
pub struct TerminalCapabilities {
    pub images: Option<ImageProtocol>,
    pub true_color: bool,
    pub hyperlinks: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct CellDimensions {
    pub width_px: u32,
    pub height_px: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct ImageDimensions {
    pub width_px: u32,
    pub height_px: u32,
}

#[derive(Debug, Clone)]
pub struct ImageRenderOptions {
    pub max_width_cells: Option<u32>,
    pub max_height_cells: Option<u32>,
    pub preserve_aspect_ratio: Option<bool>,
    pub image_id: Option<u32>,
    pub move_cursor: Option<bool>,
}

pub struct ImageCellSize {
    pub columns: u32,
    pub rows: u32,
}

// ── globals ─────────────────────────────────────────────────────────────────

static CELL_DIMS: Mutex<CellDimensions> = Mutex::new(CellDimensions {
    width_px: 9,
    height_px: 18,
});

static CAPS: Mutex<Option<TerminalCapabilities>> = Mutex::new(None);

// ── cell dimensions ─────────────────────────────────────────────────────────

pub fn get_cell_dimensions() -> CellDimensions {
    *CELL_DIMS.lock().unwrap()
}

pub fn set_cell_dimensions(dims: CellDimensions) {
    *CELL_DIMS.lock().unwrap() = dims;
}

// ── capability detection ────────────────────────────────────────────────────

/// Detect terminal capabilities from environment variables. Call once at
/// startup; result is cached in [`get_capabilities`].
pub fn detect_capabilities() -> TerminalCapabilities {
    // Default: be conservative — no images, trueColor from COLORTERM, no
    // hyperlinks (OSC 8 renders as invisible on terminals that swallow it).
    let has_true_color = std::env::var("COLORTERM")
        .map(|v| v.eq_ignore_ascii_case("truecolor") || v.eq_ignore_ascii_case("24bit"))
        .unwrap_or(false);

    let caps = TerminalCapabilities {
        images: None,
        true_color: has_true_color,
        hyperlinks: false,
    };

    // Read env once.
    let term = std::env::var("TERM").unwrap_or_default().to_lowercase();
    let term_program = std::env::var("TERM_PROGRAM")
        .unwrap_or_default()
        .to_lowercase();
    let terminal_emulator = std::env::var("TERMINAL_EMULATOR")
        .unwrap_or_default()
        .to_lowercase();

    // tmux / screen: no images, no hyperlinks (or conditional tmux).
    if std::env::var("TMUX").is_ok() || term.starts_with("tmux") {
        return TerminalCapabilities {
            images: None,
            true_color: has_true_color,
            hyperlinks: false,
        };
    }
    if term.starts_with("screen") {
        return TerminalCapabilities {
            images: None,
            true_color: has_true_color,
            hyperlinks: false,
        };
    }

    // Kitty protocol terminals.
    if std::env::var("KITTY_WINDOW_ID").is_ok() || term_program == "kitty" {
        return TerminalCapabilities {
            images: Some(ImageProtocol::Kitty),
            true_color: true,
            hyperlinks: true,
        };
    }
    if term_program == "ghostty"
        || term.contains("ghostty")
        || std::env::var("GHOSTTY_RESOURCES_DIR").is_ok()
    {
        return TerminalCapabilities {
            images: Some(ImageProtocol::Kitty),
            true_color: true,
            hyperlinks: true,
        };
    }
    if std::env::var("WEZTERM_PANE").is_ok() || term_program == "wezterm" {
        return TerminalCapabilities {
            images: Some(ImageProtocol::Kitty),
            true_color: true,
            hyperlinks: true,
        };
    }
    if term_program == "warpterminal"
        || std::env::var("WARP_SESSION_ID").is_ok()
        || std::env::var("WARP_TERMINAL_SESSION_UUID").is_ok()
    {
        return TerminalCapabilities {
            images: Some(ImageProtocol::Kitty),
            true_color: true,
            hyperlinks: true,
        };
    }

    // iTerm2
    if std::env::var("ITERM_SESSION_ID").is_ok() || term_program == "iterm.app" {
        return TerminalCapabilities {
            images: Some(ImageProtocol::ITerm2),
            true_color: true,
            hyperlinks: true,
        };
    }

    // Windows Terminal, VSCode, Alacritty: no images, but trueColor + hyperlinks.
    if std::env::var("WT_SESSION").is_ok()
        || term_program == "vscode"
        || term_program == "alacritty"
    {
        return TerminalCapabilities {
            images: None,
            true_color: true,
            hyperlinks: true,
        };
    }

    // JetBrains
    if terminal_emulator == "jetbrains-jediterm" {
        return TerminalCapabilities {
            images: None,
            true_color: true,
            hyperlinks: false,
        };
    }

    caps
}

pub fn get_capabilities() -> TerminalCapabilities {
    let mut cached = CAPS.lock().unwrap();
    cached.get_or_insert_with(detect_capabilities);
    cached.unwrap()
}

pub fn reset_capabilities_cache() {
    *CAPS.lock().unwrap() = None;
}

/// Override cached capabilities (useful in tests).
pub fn set_capabilities(caps: TerminalCapabilities) {
    *CAPS.lock().unwrap() = Some(caps);
}

// ── encoding ────────────────────────────────────────────────────────────────

const KITTY_PREFIX: &str = "\x1b_G";
const ITERM2_PREFIX: &str = "\x1b]1337;File=";

pub fn is_image_line(line: &str) -> bool {
    line.starts_with(KITTY_PREFIX)
        || line.starts_with(ITERM2_PREFIX)
        || line.contains(KITTY_PREFIX)
        || line.contains(ITERM2_PREFIX)
}

pub fn allocate_image_id() -> u32 {
    // Simple deterministic ID from monotonic counter. pi uses Math.random() but
    // in a single-process Rust app a counter avoids collisions.
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

pub fn encode_kitty(
    base64_data: &str,
    columns: Option<u32>,
    rows: Option<u32>,
    image_id: Option<u32>,
    move_cursor: Option<bool>,
) -> String {
    const CHUNK_SIZE: usize = 4096;

    let mut params: Vec<String> = vec!["a=T".into(), "f=100".into(), "q=2".into()];
    if !move_cursor.unwrap_or(true) {
        params.push("C=1".into());
    }
    if let Some(c) = columns {
        params.push(format!("c={c}"));
    }
    if let Some(r) = rows {
        params.push(format!("r={r}"));
    }
    if let Some(id) = image_id {
        params.push(format!("i={id}"));
    }

    if base64_data.len() <= CHUNK_SIZE {
        return format!("\x1b_G{};{}\x1b\\", params.join(","), base64_data);
    }

    // Multi-chunk.
    let mut chunks = String::new();
    let bytes = base64_data.as_bytes();
    let mut offset = 0;
    let mut first = true;

    while offset < bytes.len() {
        let end = (offset + CHUNK_SIZE).min(bytes.len());
        let chunk = std::str::from_utf8(&bytes[offset..end]).unwrap_or("");
        let is_last = end >= bytes.len();

        if first {
            use std::fmt::Write;
            write!(chunks, "\x1b_G{},m=1;{}\x1b\\", params.join(","), chunk).unwrap();
            first = false;
        } else if is_last {
            use std::fmt::Write;
            write!(chunks, "\x1b_Gm=0;{}\x1b\\", chunk).unwrap();
        } else {
            use std::fmt::Write;
            write!(chunks, "\x1b_Gm=1;{}\x1b\\", chunk).unwrap();
        }
        offset = end;
    }

    chunks
}

pub fn delete_kitty_image(image_id: u32) -> String {
    format!("\x1b_Ga=d,d=I,i={image_id},q=2\x1b\\")
}

pub fn delete_all_kitty_images() -> String {
    "\x1b_Ga=d,d=A,q=2\x1b\\".to_string()
}

pub fn encode_iterm2(
    base64_data: &str,
    width: Option<u32>,
    _name: Option<&str>,
    preserve_aspect_ratio: Option<bool>,
    inline: Option<bool>,
) -> String {
    let mut params: Vec<String> = vec![format!(
        "inline={}",
        if inline.unwrap_or(true) { 1 } else { 0 }
    )];
    if let Some(w) = width {
        params.push(format!("width={w}"));
    }
    // name=... requires base64-encoding the filename; skip for now (rarely used).
    if preserve_aspect_ratio == Some(false) {
        params.push("preserveAspectRatio=0".into());
    }

    format!("\x1b]1337;File={}:{}\x07", params.join(";"), base64_data)
}

// ── cell size calculation ───────────────────────────────────────────────────

pub fn calculate_image_cell_size(
    image_dims: ImageDimensions,
    max_width_cells: u32,
    max_height_cells: Option<u32>,
    cell_dims: CellDimensions,
) -> ImageCellSize {
    let max_w = max_width_cells.max(1);
    let max_h = max_height_cells.map(|h| h.max(1));
    let img_w = image_dims.width_px.max(1) as f64;
    let img_h = image_dims.height_px.max(1) as f64;

    let width_scale = (max_w as f64 * cell_dims.width_px as f64) / img_w;
    let height_scale = max_h.map(|h| (h as f64 * cell_dims.height_px as f64) / img_h);
    let scale = height_scale.map_or(width_scale, |hs| width_scale.min(hs));

    let scaled_w = img_w * scale;
    let scaled_h = img_h * scale;
    let cols = (scaled_w / cell_dims.width_px as f64).ceil() as u32;
    let rows = (scaled_h / cell_dims.height_px as f64).ceil() as u32;

    ImageCellSize {
        columns: cols.max(1).min(max_w),
        rows: max_h.map_or(rows.max(1), |h| rows.max(1).min(h)),
    }
}

pub fn calculate_image_rows(
    image_dims: ImageDimensions,
    target_width_cells: u32,
    cell_dims: CellDimensions,
) -> u32 {
    calculate_image_cell_size(image_dims, target_width_cells, None, cell_dims).rows
}

// ── render ──────────────────────────────────────────────────────────────────

pub struct RenderResult {
    pub sequence: String,
    pub rows: u32,
    pub image_id: Option<u32>,
}

pub fn render_image(
    base64_data: &str,
    image_dims: ImageDimensions,
    options: &ImageRenderOptions,
) -> Option<RenderResult> {
    let caps = get_capabilities();
    let protocol = caps.images?;

    let max_w = options.max_width_cells.unwrap_or(80);
    let cell_dims = get_cell_dimensions();
    let size = calculate_image_cell_size(image_dims, max_w, options.max_height_cells, cell_dims);

    match protocol {
        ImageProtocol::Kitty => {
            let seq = encode_kitty(
                base64_data,
                Some(size.columns),
                Some(size.rows),
                options.image_id,
                options.move_cursor,
            );
            Some(RenderResult {
                sequence: seq,
                rows: size.rows,
                image_id: options.image_id,
            })
        }
        ImageProtocol::ITerm2 => {
            let seq = encode_iterm2(
                base64_data,
                Some(size.columns),
                None,
                options.preserve_aspect_ratio,
                None,
            );
            Some(RenderResult {
                sequence: seq,
                rows: size.rows,
                image_id: None,
            })
        }
    }
}

// ── OSC 8 hyperlinks ────────────────────────────────────────────────────────

pub fn hyperlink(text: &str, url: &str) -> String {
    format!("\x1b]8;;{url}\x1b\\{text}\x1b]8;;\x1b\\")
}

// ── image dimension detection from binary headers ────────────────────────────

pub fn get_png_dimensions(data: &[u8]) -> Option<ImageDimensions> {
    if data.len() < 24 {
        return None;
    }
    if data[0] != 0x89 || data[1] != 0x50 || data[2] != 0x4e || data[3] != 0x47 {
        return None;
    }
    let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Some(ImageDimensions {
        width_px: w,
        height_px: h,
    })
}

pub fn get_jpeg_dimensions(data: &[u8]) -> Option<ImageDimensions> {
    if data.len() < 2 || data[0] != 0xff || data[1] != 0xd8 {
        return None;
    }
    let mut i = 2usize;
    while i + 9 <= data.len() {
        if data[i] != 0xff {
            i += 1;
            continue;
        }
        let marker = data[i + 1];
        if (0xc0..=0xc2).contains(&marker) {
            let h = u16::from_be_bytes([data[i + 5], data[i + 6]]) as u32;
            let w = u16::from_be_bytes([data[i + 7], data[i + 8]]) as u32;
            return Some(ImageDimensions {
                width_px: w,
                height_px: h,
            });
        }
        if i + 3 > data.len() {
            return None;
        }
        let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
        if len < 2 {
            return None;
        }
        i += 2 + len;
    }
    None
}

pub fn get_gif_dimensions(data: &[u8]) -> Option<ImageDimensions> {
    if data.len() < 10 {
        return None;
    }
    let sig = std::str::from_utf8(&data[..6]).ok()?;
    if sig != "GIF87a" && sig != "GIF89a" {
        return None;
    }
    let w = u16::from_le_bytes([data[6], data[7]]) as u32;
    let h = u16::from_le_bytes([data[8], data[9]]) as u32;
    Some(ImageDimensions {
        width_px: w,
        height_px: h,
    })
}

pub fn get_webp_dimensions(data: &[u8]) -> Option<ImageDimensions> {
    if data.len() < 30 {
        return None;
    }
    let riff = std::str::from_utf8(&data[..4]).ok()?;
    let webp = std::str::from_utf8(&data[8..12]).ok()?;
    if riff != "RIFF" || webp != "WEBP" {
        return None;
    }
    let chunk = std::str::from_utf8(&data[12..16]).ok()?;
    match chunk {
        "VP8 " => {
            if data.len() < 30 {
                return None;
            }
            let w = (u16::from_le_bytes([data[26], data[27]]) & 0x3fff) as u32;
            let h = (u16::from_le_bytes([data[28], data[29]]) & 0x3fff) as u32;
            Some(ImageDimensions {
                width_px: w,
                height_px: h,
            })
        }
        "VP8L" => {
            if data.len() < 25 {
                return None;
            }
            let bits = u32::from_le_bytes([data[21], data[22], data[23], data[24]]);
            let w = (bits & 0x3fff) + 1;
            let h = ((bits >> 14) & 0x3fff) + 1;
            Some(ImageDimensions {
                width_px: w,
                height_px: h,
            })
        }
        "VP8X" => {
            if data.len() < 30 {
                return None;
            }
            let w = data[24] as u32 | (data[25] as u32) << 8 | (data[26] as u32) << 16;
            let h = data[27] as u32 | (data[28] as u32) << 8 | (data[29] as u32) << 16;
            Some(ImageDimensions {
                width_px: w + 1,
                height_px: h + 1,
            })
        }
        _ => None,
    }
}

pub fn get_image_dimensions(data: &[u8], mime_type: &str) -> Option<ImageDimensions> {
    match mime_type {
        "image/png" => get_png_dimensions(data),
        "image/jpeg" => get_jpeg_dimensions(data),
        "image/gif" => get_gif_dimensions(data),
        "image/webp" => get_webp_dimensions(data),
        _ => None,
    }
}

pub fn image_fallback(
    mime_type: &str,
    dims: Option<ImageDimensions>,
    filename: Option<&str>,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(f) = filename {
        parts.push(f.to_string());
    }
    parts.push(format!("[{mime_type}]"));
    if let Some(d) = dims {
        parts.push(format!("{}x{}", d.width_px, d.height_px));
    }
    format!("[Image: {}]", parts.join(" "))
}
