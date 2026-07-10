# Tasks — c459-enhance-diff-edit-line-syntax

- [x] 1. Delta specs：`package-tui-diff` 增补 edit 格式 + SBS 行号 requirements
- [x] 2. `DiffInput::EditText` 解析 + compact edit gutter 渲染
- [x] 3. SBS `format_sbs_cell` 使用行号（pad by max）；空半栏无伪号
- [x] 4. 可选 `DiffTheme::highlight_line`（默认 identity）
- [x] 5. 单测：EditText 解析、SBS 行号可见、窄宽回退 unified
- [x] 6. `agent_demo`：edit 块样例；更新 `design/diff-block.md`
- [x] 7. `cargo test -p xylitol-tui`；`llman sdd validate c459-enhance-diff-edit-line-syntax --strict --no-interactive`
