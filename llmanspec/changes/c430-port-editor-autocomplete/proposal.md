---
change_id: c430-port-editor-autocomplete
title: "port editor autocomplete — SelectList popup + Tab/trigger/debounce + updateAutocomplete (pi editor.ts → Rust)"
status: draft
priority: 430
depends_on: ["c420-port-autocomplete-debounce", "c425-port-editor-core"]
author: agent
---

# c430-port-editor-autocomplete

## Why

c425 补齐了编辑核心（VisualLine/sticky/PasteBurst/history），但 autocomplete 集成完全缺失。pi 的 editor 在 `/` 起始、`@` 触发、Tab 补全时会弹出 SelectList popup，光标移动时实时更新。xy 408→540 行的 editor 没有任何 autocomplete 管线，这是 editor(6.4) 的最终缺口。

### 证据

pi editor.ts autocomplete 管线（~450 行）：
- `insertCharacter` → auto-trigger 检测（`/` at start → slash cmds, `@`/provider triggers → attachments）（:1084-1140）
- `requestAutocomplete` → debounce 决定（0ms for Tab/force/slash, 20ms for @ attach），顺序任务 startToken 去重（:2148-2174）
- `startAutocompleteRequest` → 链式 async 任务 + 快照对比（:2176-2198）
- `runAutocompleteRequest` → 调 provider.getSuggestions + post-await 快照验证 + applyAutocompleteSuggestions（:2225-2280）
- `applyAutocompleteSuggestions` → create SelectList + getBestMatch（:2292-2302）
- `handleInput` 内 autocompleteState 分支：Tab/Enter 确认、up/down 选择、cancel 退出（:653-712）
- `render` → 追加 autocompleteList render 输出（:579-585）
- `updateAutocomplete` → 编辑/光标移动后刷新 picker（insert/backspace/forwardDelete/moveCursor 调用）（:1140/1328/1693/1832）
- `cancelAutocomplete` → cancel 请求 + 清 UI（:2318-2322）

xy 现状：
- `DebouncedAutocomplete` wrapper 就位（c420）
- `SelectList` widget 就位
- `CombinedAutocompleteProvider` async/sync 双路径就位
- 缺：所有上面 10 个方法 + render/handle_input 集成

### 往期验证

- c420 `DebouncedAutocomplete`: 250ms debounce + CancellationToken 验证通过
- c425 `SelectList` 在 tui_integration_test 和 snapshot_test 中已工作
- c405 第 1 层 TuiTestHarness 可用于 autocomplete popup 交互测试

## What Changes

1. **autocomplete 字段**（Editor 新加 8 个字段）：
   - `autocomplete_provider: Option<CombinedAutocompleteProvider>`
   - `autocomplete_trigger_chars: Vec<char>`
   - `autocomplete_list: Option<SelectList>`
   - `autocomplete_state: Option<AutocompleteMode>`（Regular | Force）
   - `autocomplete_prefix: String`
   - `autocomplete_max_visible: usize`
   - `autocomplete_abort: Option<CancellationToken>`
   - `autocomplete_start_token: usize`

2. **模式切换**：
   - `EditorOptions` 加 `spawn_handle: Option<tokio::runtime::Handle>` —— 有 handle 时用 async+debounce，无 handle 时用 sync 即时补全
   - `set_autocomplete_provider(provider)` 公有方法

3. **触发管线**（~10 个私有方法）：
   - `try_trigger_autocomplete(explicit_tab)` — Tab/字符 统一入口
   - `handle_tab_completion()` — 单结果自动应用
   - `request_autocomplete(force, explicit_tab)` — 决定 debounce ms，串行 startToken
   - `start_autocomplete_request(start_token, force)` — spawn async task
   - `run_autocomplete_request(request_id, ct, snapshot_*, force)` — 调 provider + 快照验证 + apply
   - `apply_autocomplete_suggestions(suggestions, mode)` — 建 SelectList + bestMatch
   - `cancel_autocomplete_request()` / `clear_autocomplete_ui()` / `cancel_autocomplete()`
   - `update_autocomplete()` — 编辑/光标后刷新
   - `create_autocomplete_list(prefix, items)` — SelectList 工厂
   - `get_best_autocomplete_match_index(items, prefix)` — 优先精确匹配
   - `get_autocomplete_debounce_ms(force, explicit_tab)` → 0 | 20 | 250

4. **render 集成**：autocompleteList render 结果追加在 editor 输出后

5. **handle_input 集成**：
   - autocompleteState 活跃时：Tab/confirm 应用 → cancelAutocomplete；up/down 导航；cancel 关闭
   - insertCharacter 末尾：auto-trigger
   - backspace/forwardDelete/moveCursor 末尾：updateAutocomplete

6. **lib.rs 导出** `AutocompleteMode`

7. **测试（c405 第 1 层）**：
   - sync：Tab 触发 slash 命令补全 → SelectList popup；Enter 确认 → 应用；cancel 关闭
   - sync：@ 触发 attachment 补全
   - async（需 tokio runtime）：debounce 20ms 延迟补全
   - 更新 autocomplete_test.rs 加 editor 集成测试

## Capabilities

- `editor-autocomplete`（新建）

## Impact

- `packages/xylitol-tui/src/components/editor.rs`（修改，+~350 行）
- `packages/xylitol-tui/src/lib.rs`（导出 `AutocompleteMode`）
- `packages/xylitol-tui/Cargo.toml`（可能加 tokio rt feature for editor）
- `packages/xylitol-tui/tests/autocomplete_test.rs`（追加或新建 editor_ac_test.rs）
- `llmanspec/specs/editor-autocomplete/spec.toon`（新建）

## Non-goals

- 不引入 `slash-command argument completion`（pi 的 command.getArgumentCompletions 回调）——那是上层 api 对接
- 不实现 pi 的 `isAutocompleteRequestCurrent` 四条件快照对比的完整版本（editor 文本不变性太强，简化快照即可）
- 不改 editor 的 pub API（on_submit/on_change/disable_submit 保留）
