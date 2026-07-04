# NOTICE — vendored ratatui-markdown

This directory (`src/app/tui/vendor/ratatui_markdown/`) contains a vendored
copy of the **ratatui-markdown** library, derived from:

- **Source**: https://github.com/celestia-island/ratatui-markdown
- **Original author**: langyo <langyo.china@gmail.com>
- **Copyright**: (c) 2026 langyo
- **License**: SySL-1.0 (see `LICENSE` in this directory for the full text)

## Adaptations made for xylitol

- Adapted all `use ratatui::` imports to `ratatui_core::` (xylitol uses the
  split-crate `ratatui-core`/`ratatui-widgets`, not the umbrella `ratatui`).
- Excluded upstream modules not needed by xylitol: `mermaid`, `scroll`,
  `tree`, `preview`, `viewer`, `text_input`, `image`, `constants/box_chars`.
- Renamed the `gen` field (Rust 2024 reserved keyword) to `generation_field`.
- Removed explicit `ref` patterns (Rust 2024 implicit borrowing).
- Added `highlight/syntect_bridge.rs`: a syntect + two-face based
  `CodeHighlighter` implementation (replacing the upstream tree-sitter
  backend), derived from codex's `codex-rs/tui/src/render/highlight.rs`.

## MODEL DISCLOSURE (SySL requirement)

Per SySL-1.0 transparency requirements, this vendored copy and its
adaptations were integrated with assistance from AI coding assistants:

- **Human author**: straydragon
- **AI models used**: ZCode (built-in model, Claude family)
- **Purpose**: type-path adaptation, edition-2024 fixes, syntect backend port
