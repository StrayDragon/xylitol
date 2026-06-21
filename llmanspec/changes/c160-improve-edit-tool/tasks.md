# c160-improve-edit-tool: Tasks

## Implementation

- [ ] 在 `patch.rs` 中添加 `normalize_for_fuzzy_match()` — Unicode 规范化 + 智能引号/破折号替换
- [ ] 在 `patch.rs` 中添加 `detect_line_ending()` — CRLF/LF 检测
- [ ] 在 `patch.rs` 中添加 `restore_line_endings()` — 行尾还原
- [ ] 在 `patch.rs` 中添加 `find_span()` — 多行跨段滑动窗口匹配
- [ ] 在 `edit.rs` 中集成模糊匹配为回退策略
- [ ] 在 `edit.rs` 中添加 `compute_diff()` — 使用 `similar` crate 计算差异输出

## Testing

- [ ] 单元测试 — 模糊匹配（智能引号、破折号、尾部空白、Unicode）
- [ ] 单元测试 — 行尾检测与还原（CRLF ↔ LF）
- [ ] 单元测试 — 跨段匹配
- [ ] 单元测试 — 差异输出格式
- [ ] 现有 edit 测试继续通过

## Verification

- [ ] `cargo check`
- [ ] `cargo test --lib`
- [ ] `llman sdd validate c160-improve-edit-tool`
