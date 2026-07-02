# c380 Design — TUI 固定状态行

## 布局（三段）

```
[MutableLine: thinking/reply text]        ← 顶，透明 bg，flush against scrollback（c365 不变）
{spinner} Working…      Turn 2        gpt-4o   ← 中，StatusLine，固定 1 行（新增）
└── left (左对齐) ──┘  └ center (居中) ┘└ right (右对齐) ┘
────────────────                          ← panel top border
<input>                                   ← InputPrompt（不变）
────────────────                          ← panel bottom border
```

StatusLine 始终 1 行（idle 也在），内部是**三段式状态栏**（对标 vim airline / VSCode status bar）：

- **left（左对齐）**：`spinner`（固定在 left 段首位）+ 活动标签（`Working…` / `Running {tool}` / `Ready`）。
- **center（居中）**：`Turn {n}`（streaming 时）/ 空（idle）。
- **right（右对齐）**：`{model_name}`。

**扩展性**（用户 2026-07-02 要求）：三段布局是骨架，未来加新状态项（token 计数、耗时、上下文窗口占用等）只需往对应段加内容，不改 widget 渲染逻辑。spinner 固定 left 首位是唯一硬约束。

TAIL_HEIGHT 5 → 6（mutable 2 + status 1 + panel 3）。

## 数据来源（TuiApp 现有 + 新增）

StatusLine 是数据驱动的：`TuiApp` 暴露一个 `status_segments()` 返回三段内容，widget 只负责按左/中/右布局渲染。这样未来扩展不改 widget。

| 段 | 内容 | 来源 | 现有? |
|---|---|---|---|
| left | spinner glyph | `app.spinner_idx()` + Tick 推进 | ✅ |
| left | 活动标签 | `app.is_streaming()` + `app.tool_status()` | ✅（tool_status 字段） |
| center | Turn {n} | **新增** `turn_index: Option<u32>`（TurnStart 设） | 新增 |
| right | 模型名 | **新增** `model_name: Option<String>`（ModelSelect 设） | 新增 |

新增字段：`turn_index: Option<u32>`、`model_name: Option<String>`。`handle_xy_event` 新增分支：`TurnStart { turn_index }` → 记 turn_index；`ModelSelect { model_id }` → 记 model_id。

`status_segments()` 返回 `StatusSegments { left: Vec<Span>, center: Option<String>, right: Option<String> }`（left 是 Span 列表以容纳 spinner glyph + 标签的不同样式；center/right 是 String）。

## StatusLine widget（新增 components/status_line.rs）

- 始终渲染 1 行。
- 用 `Layout::horizontal` 把宽度切成三段（left / center / right），每段按对齐渲染（left 左对齐、center 居中、right 右对齐）。
- spinner 用 `Spinner::new(app.spinner_idx())` 渲染到 left 段首位，紧接活动标签 Span。
- 三段内容来自 `app.status_segments()`，widget 不自己 match 状态——纯布局渲染。
- 颜色走 theme token（spinner/thinking/normal），不硬编码。

## SPINNER 单一来源

spinner.rs 的 `pub const SPINNER` 为 SSOT；app.rs 删本地 `SPINNER`，改 `use crate::app::tui::components::spinner::SPINNER`（tick_spinner 仍用）。

## Tail 改动

render 顺序：MutableLine（顶，available_above 减 1 给 status）→ StatusLine（panel 上方 1 行）→ BottomPanel（底 3 行）。`input_cursor_position` 的 y 计算不变（panel 仍在底部，cursor 在 panel 内）。

## 测试

新增 StatusLine harness（TestBackend）：idle 显示 Ready+model、streaming 显示 spinner+Working+Turn+model、tool running 显示工具名、三段对齐正确。更新既有 Tail harness 行号（+1 行偏移）。
