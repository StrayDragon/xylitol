use crate::terminal_image::{
    self, ImageDimensions, ImageRenderOptions, get_capabilities, image_fallback, render_image,
};
use crate::tui::Component;

/// Styling hooks for [`Image`].
pub struct ImageTheme {
    pub fallback_color: Box<dyn Fn(&str) -> String>,
}

/// Options for [`Image`].
#[derive(Default)]
pub struct ImageOptions {
    pub max_width_cells: Option<u32>,
    pub max_height_cells: Option<u32>,
    pub filename: Option<String>,
    /// Kitty image ID. If provided, reuses this ID (for animations/updates).
    pub image_id: Option<u32>,
}

/// Renders a base64-encoded image inline using the Kitty or iTerm2 graphics
/// protocol. Falls back to a `[Image: ...]` placeholder on terminals that don't
/// support images.
///
/// Ported from pi's `components/image.ts`.
pub struct Image {
    base64_data: String,
    mime_type: String,
    dimensions: ImageDimensions,
    theme: ImageTheme,
    options: ImageOptions,
    image_id: Option<u32>,

    cached_lines: Option<Vec<String>>,
    cached_width: Option<usize>,
}

impl Image {
    pub fn new(
        base64_data: String,
        mime_type: String,
        theme: ImageTheme,
        options: ImageOptions,
        dimensions: Option<ImageDimensions>,
    ) -> Self {
        let image_id = options.image_id;
        let dims = dimensions.unwrap_or(ImageDimensions {
            width_px: 800,
            height_px: 600,
        });
        Self {
            base64_data,
            mime_type,
            dimensions: dims,
            theme,
            options,
            image_id,
            cached_lines: None,
            cached_width: None,
        }
    }

    /// Get the Kitty image ID used by this image (if any).
    pub fn image_id(&self) -> Option<u32> {
        self.image_id
    }
}

impl Component for Image {
    fn render(&mut self, width: usize) -> Vec<String> {
        // Width in component convention is usize (chars), but image system uses u32.
        let width_u32 = width.max(2) as u32;

        if self.cached_width.is_some_and(|w| w == width)
            && let Some(ref lines) = self.cached_lines
        {
            return lines.clone();
        }

        let caps = get_capabilities();
        let max_w = self
            .options
            .max_width_cells
            .unwrap_or(60)
            .min(width_u32.saturating_sub(2))
            .max(1);
        let cell_dims = terminal_image::get_cell_dimensions();
        let default_max_h =
            ((max_w as f64 * cell_dims.width_px as f64) / cell_dims.height_px as f64).ceil();
        let default_max_h = default_max_h.max(1.0) as u32;
        let max_h = self.options.max_height_cells.unwrap_or(default_max_h);

        let mut allocated_id = false;
        if caps.images == Some(terminal_image::ImageProtocol::Kitty) && self.image_id.is_none() {
            self.image_id = Some(terminal_image::allocate_image_id());
            allocated_id = true;
        }

        let result = render_image(
            &self.base64_data,
            self.dimensions,
            &ImageRenderOptions {
                max_width_cells: Some(max_w),
                max_height_cells: Some(max_h),
                preserve_aspect_ratio: None,
                image_id: self.image_id,
                move_cursor: Some(false),
            },
        );

        let lines = match result {
            Some(r) => {
                if r.image_id.is_some() {
                    self.image_id = r.image_id;
                } else if allocated_id {
                    // restore
                    self.image_id = None;
                }

                let rows = r.rows as usize;
                if caps.images == Some(terminal_image::ImageProtocol::Kitty) {
                    // Kitty C=1 prevents cursor movement. One line for the
                    // sequence, plus empty lines for the image height.
                    let mut v: Vec<String> = vec![r.sequence];
                    v.resize(rows, String::new());
                    v
                } else {
                    // iTerm2: first rows-1 empty, last line moves up then
                    // draws the image.
                    let mut v: Vec<String> = Vec::new();
                    v.resize(rows.saturating_sub(1), String::new());
                    if rows > 1 {
                        v.push(format!("\x1b[{}A{}", rows - 1, r.sequence));
                    } else {
                        v.push(r.sequence);
                    }
                    v
                }
            }
            None => {
                let fallback = image_fallback(
                    &self.mime_type,
                    Some(self.dimensions),
                    self.options.filename.as_deref(),
                );
                vec![(self.theme.fallback_color)(&fallback)]
            }
        };

        self.cached_lines = Some(lines.clone());
        self.cached_width = Some(width);
        lines
    }

    fn handle_input(&mut self, _event: crate::tui::InputEvent) {}
    fn invalidate(&mut self) {
        self.cached_lines = None;
        self.cached_width = None;
    }
}
