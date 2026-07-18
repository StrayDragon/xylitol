# Tasks: c1350

## Specs

- [x] live specs：edit display_diff hunk + TUI cap
- [x] attach

## Implement

- [x] 修 `edit` old/new 为整文件；`generate_display_diff` context + 行封顶
- [x] scrollback Diff 渲染封顶 + 大 diff 关 word-level
- [x] 单测

## Verify

- [x] `cargo test -q --lib infra::tools::patch`
- [x] `cargo test -q --lib app::tui::widgets::scrollback`
- [x] validate
