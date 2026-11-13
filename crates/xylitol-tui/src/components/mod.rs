pub mod cancellable_loader;
pub mod choice_prompt;
pub mod container;
pub mod diff;
pub mod editor;
pub mod expandable_output;
pub mod input;
pub mod loader;
pub mod markdown;
pub mod mermaid_ascii;
pub mod panel;
pub mod panel_region;
pub mod scroll_region;
pub mod select_list;
pub mod settings_list;
pub mod spacer;
pub mod tabs;
pub mod text;
pub mod tree_selector;
pub mod truncated_text;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LastAction {
    Kill,
    Yank,
    TypeWord,
}
