# Design — c452-demo-highlight-pipeline

## Approach

Optional Cargo feature `highlight` on `xylitol-tui` pulls `syntect` + `two-face` (fancy regex, no oniguruma). Default package build stays light.

`highlight::syntect_bridge::highlight_code(code, lang) -> Vec<String>` returns ANSI lines for `MarkdownTheme.highlight_code`.

Safety: skip / plain fallback when `code.len() > MAX_BYTES` or line count > `MAX_LINES`.

Theme: Catppuccin Mocha (matches DESIGN dark tokens).

## Demo

Assistant `Message` entries render via `Markdown` with injected highlighter when feature enabled. Seed includes a ```rust fence. Tests assert ANSI present under `--features highlight`.
