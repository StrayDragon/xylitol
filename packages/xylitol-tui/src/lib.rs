pub mod autocomplete;
pub mod autocomplete_fd;
pub mod clock;
pub mod completion;
pub mod components;
pub mod editor_component;
pub mod fuzzy;
pub mod highlight;
pub mod keybindings;
pub mod keys;
pub mod kill_ring;
pub mod paste_burst;
pub mod terminal;
pub mod terminal_colors;
pub mod terminal_image;
pub mod tui;
pub mod undo_stack;
pub mod utils;
pub mod word_navigation;

pub use autocomplete::{
    AutocompleteItem, AutocompleteProvider, AutocompleteSuggestions, CombinedAutocompleteProvider,
    DebouncedAutocomplete, SlashCommand, extract_at_prefix, parse_path_prefix,
};
pub use autocomplete_fd::{build_fd_path_query, walk_directory_with_fd};
pub use clock::{Clock, MockClock, SystemClock};
pub use completion::{
    AtPathSource, CompletionContext, CompletionMatch, CompletionRegistry, CompletionSource,
    SlashCommandSource, sources_from_combined,
};
pub use components::cancellable_loader::CancellableLoader;
pub use components::container::Container;
pub use components::diff::{
    Diff, DiffInput, DiffOptions, DiffTheme, generate_edit_text, render_diff_lines,
};
pub use components::editor::{AutocompleteMode, Editor, EditorOptions, EditorTheme, VisualLine};
pub use components::input::Input;
pub use components::loader::{Loader, LoaderIndicatorOptions};
pub use components::markdown::{DefaultTextStyle, Markdown, MarkdownOptions, MarkdownTheme};
pub use components::panel::Panel;
pub use components::select_list::{
    SelectItem, SelectList, SelectListLayoutOptions, SelectListTheme,
    SelectListTruncatePrimaryContext,
};
pub use components::settings_list::{
    SettingItem, SettingsList, SettingsListOptions, SettingsListTheme,
};
pub use components::spacer::Spacer;
pub use components::text::Text;
pub use components::truncated_text::TruncatedText;
pub use editor_component::EditorComponent;
pub use fuzzy::{FuzzyMatch, fuzzy_filter, fuzzy_match};
pub use highlight::{highlight_code, highlight_code_owned};
pub use keybindings::{
    Keybinding, KeybindingConflict, KeybindingDefinition, KeybindingDefinitions, KeybindingsConfig,
    KeybindingsManager, create_default_definitions, set_keybindings, with_keybindings,
};
pub use keys::{
    KeyId, decode_printable_key, is_key_release, is_key_repeat, matches_key, matches_key_event,
    parse_key, printable_from_key_event, set_kitty_protocol_active,
};
pub use paste_burst::PasteBurst;
pub use terminal::{CrosstermTerminal, Terminal, parse_kitty_flags};
pub use terminal_colors::{
    RgbColor, TerminalColorScheme, is_osc11_background_color_response,
    parse_osc11_background_color, parse_terminal_color_scheme_report,
};
pub use terminal_image::{hyperlink, is_image_line};
pub use tui::{
    Component, Focusable, InputEvent, InputListenerResult, OverlayAnchor, OverlayHandle,
    OverlayMargin, OverlayOptions, RenderError, SizeValue, TUI,
};
pub use utils::{
    ExtractedSegments, extract_ansi_code, extract_segments, is_punctuation_char,
    is_whitespace_char, slice_by_column, slice_by_column_strict, truncate_to_width, visible_width,
    wrap_text_with_ansi,
};
