# c160-improve-edit-tool: Tasks

## Implementation

- [x] 在 `patch.rs` 中添加 `normalize_for_fuzzy_match()` — Unicode 规范化 + 智能引号/破折号/Unicode 空格替换 + 尾部空白剥离
- [x] 在 `patch.rs` 中添加 `detect_line_ending()` — CRLF/LF 检测
- [x] 在 `patch.rs` 中添加 `restore_line_endings()` — 行尾还原
- [x] 在 `patch.rs` 中增强 `fuzzy_find()` — 使用 `normalize_for_fuzzy_match` 作为 2 级回退
- [x] 差异输出已存在（`generate_unified_diff`、`generate_display_diff` 使用 `similar` crate）

## Testing

- [x] 单元测试 — 模糊匹配（智能引号、破折号、Unicode 空格、尾部空白）
- [x] 单元测试 — 行尾检测与还原（CRLF ↔ LF）
- [x] 现有 edit 测试继续通过（434 lib tests, +11 new patch tests）

## Verification

- [x] `cargo check` — 0 errors
- [x] `cargo test --lib` — 434 passed
- [x] `llman sdd validate c160-improve-edit-tool`
