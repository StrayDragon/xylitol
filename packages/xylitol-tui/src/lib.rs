pub mod autocomplete;
pub mod clock;
pub mod components;
pub mod editor_component;
pub mod fuzzy;
pub mod keybindings;
pub mod keys;
pub mod kill_ring;
pub mod paste_burst;
pub mod stdin_buffer;
pub mod terminal;
pub mod terminal_colors;
pub mod terminal_image;
pub mod tui;
pub mod undo_stack;
pub mod utils;
pub mod word_navigation;

pub use clock::{Clock, MockClock, SystemClock};
pub use components::cancellable_loader::CancellableLoader;
pub use components::image::{Image, ImageOptions, ImageTheme};
pub use components::input::Input;
pub use components::loader::{Loader, LoaderIndicatorOptions};
pub use components::panel::Panel;
pub use components::settings_list::{
    SettingItem, SettingsList, SettingsListOptions, SettingsListTheme,
};
pub use components::spacer::Spacer;
pub use components::text::Text;
pub use components::truncated_text::TruncatedText;
pub use editor_component::EditorComponent;
pub use fuzzy::{FuzzyMatch, fuzzy_filter, fuzzy_match};
pub use keybindings::{
    Keybinding, KeybindingConflict, KeybindingDefinition, KeybindingDefinitions, KeybindingsConfig,
    KeybindingsManager, create_default_definitions, set_keybindings, with_keybindings,
};
pub use keys::{
    KeyId, decode_printable_key, is_key_release, is_key_repeat, matches_key, parse_key,
    set_kitty_protocol_active,
};
pub use paste_burst::PasteBurst;
pub use stdin_buffer::{StdinBuffer, StdinBufferEvent};
pub use terminal::{CrosstermTerminal, Terminal, parse_kitty_flags};
pub use terminal_colors::{
    RgbColor, TerminalColorScheme, is_osc11_background_color_response,
    parse_osc11_background_color, parse_terminal_color_scheme_report,
};
pub use terminal_image::{
    CellDimensions, ImageDimensions, ImageProtocol, TerminalCapabilities,
    calculate_image_cell_size, calculate_image_rows, detect_capabilities, encode_iterm2,
    encode_kitty, get_capabilities, hyperlink, is_image_line, render_image,
    reset_capabilities_cache, set_capabilities, set_cell_dimensions,
};
pub use tui::{
    Component, Focusable, OverlayAnchor, OverlayMargin, OverlayOptions, RenderError, SizeValue, TUI,
};
pub use utils::{
    ExtractedSegments, extract_ansi_code, extract_segments, is_punctuation_char,
    is_whitespace_char, slice_by_column, slice_by_column_strict, truncate_to_width, visible_width,
    wrap_text_with_ansi,
};
