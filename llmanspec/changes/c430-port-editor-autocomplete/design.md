# Design — c430-port-editor-autocomplete

## 决策 1：同步 vs 异步 autocomplete

| 方案 | 做法 | 结论 |
|---|---|---|
| **A. 同步优先，startToken 去重** | Editor 直接调 provider.get_suggestions() 同步方法，用 autocomplete_start_token 递增丢弃过期请求 | ✅ 采用 |
| B. async + tokio runtime handle | Editor::new 接受 tokio Handle，spawn async tasks | ❌ 拒绝（c430） |

### 理由

- Editor 在 `Component::render/handle_input` 的同步接口中运行。引入 async 需要 runtime handle，这在库层面是重量依赖。
- pi 的 async 主要用于 20ms debounce 和 AbortController 取消。sync 路径中，startToken 递增提供了等价取消：`if start_token != self.autocomplete_start_token { return; }`。
- 20ms 和 250ms debounce 可在 sync 路径省略——对 Tab 补全是 0ms，对字符触发是即时。用户体验差异不大（Tab 是显式操作，不会频率触发）。
- c420 的 `DebouncedAutocomplete` 留作将来 async 后门。当 host 提供 tokio handle 时，可以通过 `Arc<DebouncedAutocomplete>` 注入 editor。

### 后果

- `get_autocomplete_debounce_ms` 始终返回 0
- 不需要 `tokio::runtime::Handle`
- 不需要 `CancellationToken` 字段
- `startAutocompleteRequest` 直接同步调用 provider 而非 spawn

## 决策 2：SelectList 创建策略

| 方案 | 做法 | 结论 |
|---|---|---|
| **A. 内联 SelectList 构造** | editor.rs 直接 `use crate::components::select_list::{...}` | ✅ 采用 |
| B. 通过 trait 抽象 | 定义 `AutocompleteList: Component + ...` trait | ❌ 拒绝 |

### 理由

- SelectList 是稳定的内部 widget，不对外 trait-bound
- pi 也是直接 `new SelectList(...)`（:2121）
- 额外 trait 只为 editor 的单个使用时过度设计

## 决策 3：EditorTheme 扩展

pi 的 `EditorTheme { borderColor, selectList }`。xy 需要加 `select_list_theme: SelectListTheme`。

### 后果

- `EditorTheme` 新增 `select_list_theme: SelectListTheme`
- 使用 `SelectListTheme::default()` 作为默认值向后兼容

## 非目标

- 不实现 slash-command argument completion（command.getArgumentCompletions 回调）
- 不实现 async debounce（留作将来 tokio handle 注入后门）
- 不修改 Editor pub API（on_submit/on_change/disable_submit 保留）
- 不实现 pi 的 attachment debounce pattern 正则匹配
