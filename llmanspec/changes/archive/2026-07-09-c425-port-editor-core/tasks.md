# c425 Tasks — editor core VisualLine+stickyColumn+pageScroll+history+PasteBurst

> 重写 editor.rs 从 408 → ~540 行（含 20 测试），+20 新单测。c405 第 1 层验证。

## 阶段 1：VisualLine 数据模型 + map（1.5h）

- [x] 1.1 新增 `VisualLine` struct：`logical_line: usize, start_col: usize, len: usize`
- [x] 1.2 实现 `build_visual_line_map(&self, width: usize) -> Vec<VisualLine>`
- [x] 1.3 实现 `find_current_visual_line(&self, vls: &[VisualLine]) -> usize`
- [x] 1.4 实现 `find_visual_line_at(&self, vls: &[VisualLine], line: usize, col: usize) -> usize`
- [x] 1.5 `cargo check -p xylitol-tui` 通过

## 阶段 2：Sticky column + move_to_visual_line + pageScroll（2h）

- [x] 2.1 `Editor` 加 `preferred_visual_col: Option<usize>` + `snapped_from_cursor_col: Option<usize>`
- [x] 2.2 `set_cursor_col` 清除 preferred/snapped
- [x] 2.3 实现 `compute_vertical_move_column(current, src_max, tgt_max) -> usize`（P/S/T/U 决策表）
- [x] 2.4 实现 `move_to_visual_line(vls, from_vl, to_vl)`
- [x] 2.5 重写 `move_cursor(dl, dc)` 使用 `build_visual_line_map`+`move_to_visual_line`
- [x] 2.6 实现 `page_scroll(direction)`
- [x] 2.7 `EditorOptions` 加 `terminal_rows: usize`（默认 24）
- [x] 2.8 render() max_vis 改用 `max(5, terminal_rows * 30 / 100)` 替代硬编码
- [x] 2.9 handle_input 中 pageUp/pageDown 调用 `page_scroll` 而非 `move_cursor(-5,0)`
- [x] 2.10 `cargo check -p xylitol-tui` 通过

## 阶段 3：History 导航重构（1h）

- [x] 3.1 新增 `set_text_internal(text, cursor_placement: CursorPlacement)` enum+方法
- [x] 3.2 `hist_nav` → `navigate_history(direction: isize)`，对齐 pi 语义
- [x] 3.3 新增 `exit_history_browsing()` 方法
- [x] 3.4 在编辑动作入口调用 `exit_history_browsing()`（insert_ch/backspace/fwd_delete/newline/undo/yank/yank_pop/paste/submit/set_text/del_* 等）
- [x] 3.5 `cargo check -p xylitol-tui` 通过

## 阶段 4：PasteBurst 集成（0.5h）

- [x] 4.1 `Editor` 加 `paste_burst: PasteBurst` + `clock: Box<dyn Clock>`
- [x] 4.2 `insert_ch` → `paste_burst.on_plain_char(clock.now())`
- [x] 4.3 submit/newline → `paste_burst.should_insert_newline_instead_of_submit(clock.now())` → 插入换行
- [x] 4.4 非普通字符/非 Enter → `paste_burst.reset()`
- [x] 4.5 `Editor::new()` 接受 `clock: Box<dyn Clock>`
- [x] 4.6 `cargo check -p xylitol-tui` 通过

## 阶段 5：测试（第 1 层，2h）

- [x] 5.1 编辑器内嵌 20 个 `#[test]`（含 7 个旧回归 + 13 个新测试覆盖 VL/sticky/pageScroll/PasteBurst/history）
- [x] 5.2 ed01：VL map 测试 — wrap、空行、多逻辑行、光标在 wrap 末尾
- [x] 5.3 ed02：sticky column 测试 — 上下保留列、水平移动清除 preferred
- [x] 5.4 ed03：page scroll 测试 — 页大小移动
- [x] 5.5 ed04：PasteBurst 测试 — Enter 抑制、非打印键 reset
- [x] 5.6 ed05：history 测试 — draft 恢复、编辑退出浏览、placement
- [x] 5.7 回归：7 个旧测试通过（empty/insert_get/backspace_del/newline_split/undo_test/word_lr/submit_cb）

## 阶段 6：验证（0.5h）

- [x] 6.1 `cargo test -p xylitol-tui` 全量通过（243 tests）
- [x] 6.2 `cargo clippy -p xylitol-tui --all-targets -- -D warnings` clean
- [x] 6.3 `llman sdd validate c425-port-editor-core --strict` 通过
- [x] 6.4 更新 `_HANDOFF.md` 已知差距表（editor 标 updated）

## 校验命令

```bash
cargo test -p xylitol-tui
cargo clippy -p xylitol-tui --all-targets -- -D warnings
llman sdd validate c425-port-editor-core --strict
```
