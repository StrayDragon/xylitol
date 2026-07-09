# c430 Tasks — editor autocomplete 集成

> 在 editor 中集成 autocomplete SelectList popup + Tab 触发 + 光标刷新。~350 行新增，0 新增独立测试（复用 editor inline tests）。

## 阶段 1：字段与构造函数（0.5h）

- [x] 1.1 `Editor` 加 7 个 autocomplete 字段（provider/list/state/prefix/max_visible/start_token/trigger_chars）
- [x] 1.2 `Editor::new()` 初始化 autocomplete 字段
- [x] 1.3 `EditorTheme` 加 `select_list_theme: SelectListTheme`（默认 `SelectListTheme::default()`）
- [x] 1.4 `cargo check` 通过

## 阶段 2：autocomplete 方法（1.5h）

- [x] 2.1 `set_autocomplete_provider(Option<CombinedAutocompleteProvider>)`
- [x] 2.2 `try_trigger_autocomplete(explicit_tab)` → `request_autocomplete(force, explicit_tab)`
- [x] 2.3 `handle_tab_completion()` — Tab 入口，slash/force 分流
- [x] 2.4 `request_autocomplete(force, explicit_tab)` — startToken 递增 + 同步即时调用
- [x] 2.5 `start_autocomplete_request(start_token, force, explicit_tab)` — 调用 provider + 单结果 auto-apply
- [x] 2.6 `apply_autocomplete_suggestions(suggestions, mode)` — 建 SelectList + bestMatch
- [x] 2.7 `get_best_autocomplete_match_index(items, prefix)` — 精确匹配 > 前缀匹配
- [x] 2.8 `update_autocomplete()` — 编辑/光标后刷新
- [x] 2.9 `cancel_autocomplete()` / `cancel_autocomplete_request()` / `clear_autocomplete_ui()`
- [x] 2.10 `handle_autocomplete_on_edit()` — 单向门调用 update
- [x] 2.11 `cargo check` 通过

## 阶段 3：render/handle_input 集成（1h）

- [x] 3.1 `insert_ch` 末尾添加 auto-trigger 逻辑（`/` at line-start → 斜杠补全, trigger chars → 附件补全）
- [x] 3.2 `handle_input` autocompleteState 活跃时路由 Tab/Enter/up/down/cancel 到 autocomplete 管线
- [x] 3.3 backspace/forwardDelete → `handle_autocomplete_on_edit()`
- [x] 3.4 move_cursor/wordLeft/wordRight/arrow keys → `handle_autocomplete_on_edit()`
- [x] 3.5 `render` 追加 autocompleteList render 输出在下边框之后
- [x] 3.6 `cargo check` 通过

## 阶段 4：验证（0.5h）

- [x] 4.1 `cargo test -p xylitol-tui` 全量通过（243 tests）
- [x] 4.2 `cargo clippy -p xylitol-tui --all-targets -- -D warnings` clean
- [x] 4.3 `llman sdd validate c430-port-editor-autocomplete --strict` 通过
- [x] 4.4 更新 `_HANDOFF.md` 已知差距表（editor autocomplete 标完成）

## 校验命令

```bash
cargo test -p xylitol-tui
cargo clippy -p xylitol-tui --all-targets -- -D warnings
llman sdd validate c430-port-editor-autocomplete --strict
```
