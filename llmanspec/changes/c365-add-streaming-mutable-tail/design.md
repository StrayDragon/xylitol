# c365 Design — 流式 mutable-last-line + 组件化

## 1. 核心思路

`Viewport::Inline` + `insert_before` 是单向的：commit 进 scrollback 就永久固定。mutable（未换行的尾部）**不进 scrollback**，而在 viewport 内每帧重绘——旧内容被新帧覆盖，视觉上原地生长。

布局（ratatui buffer 内，非 escape 路线）：

```
…scrollback（已 commit 正文）…
[mutable 未换行尾部]        ← viewport 顶，透明 bg，紧贴 scrollback，每帧重绘
─── panel 上 border ───
<输入框>                    ← panel 底锚
─── panel 下 border ───
```

## 2. 流程

### 2.1 思考阶段（ThinkingDelta）

1. Turn 开始 → `thinking_phase = true`，mutable 显示 `Thinking…`（灰色占位）
2. `ThinkingDelta` 到达 → `thinking_buf` 累积，完整行 drain 为 `RenderedLine::ThinkingText`（灰色）立即 `insert_before` commit 到 scrollback
3. 未换行尾部 = mutable 行（灰色，`MutableKind::Thinking` → `palette.thinking()`）

### 2.2 文字阶段（TextDelta）

1. 首个 `TextDelta` → flush `thinking_buf` 残留为最后一行 `ThinkingText`，`thinking_phase = false`
2. 后续 `TextDelta` → `stream_buf` 累积，完整行 drain 为 `RenderedLine::AssistantText`（正常色）commit
3. 未换行尾部 = mutable 行（正常色，`MutableKind::Text` → `palette.assistant()`）

### 2.3 工具执行

无流式文字时 mutable 显示 `tool_status`（`MutableKind::Tool` → `palette.tool()`）：`⚙ running bash` / `✓ bash done`。

### 2.4 TurnEnd

Flush `thinking_buf` 和 `stream_buf` 的残留，mutable 消失，面板恢复 3 行固定底部。

### 2.5 换行门控

`StreamBuffer { buffer, committed_len }`：`push(delta)` 累积 token，`drain_complete_lines()` 返回换行边界内的完整行，`pending_tail()` 返回未换行尾部。codex 极简版（无 markdown 解析、无 table holdback）。

## 3. 组件（src/app/tui/components/）

| widget | 职责 | 注 |
|---|---|---|
| `TranscriptLine` | `RenderedLine → Buffer`（wrap + CJK，含 `ThinkingText` 灰） | insert_before 与 TestBackend 共用 |
| `MutableLine` | 未换行尾部，caller 传 `Style`，透明 bg，top-anchored | thinking 灰 / text 正常 / tool 黄 |
| `InputPrompt` | 输入框，无 `❯` 前缀，MVP 单行 | CJK 显示宽度光标 |
| `BottomPanel` | bordered + panel_bg 容器，只含 InputPrompt | 固定 3 行（border+input+border） |
| `Tail` | 组合 MutableLine（顶）+ BottomPanel（底） | `draw_tail_frame` 退化为此 |
| `Spinner` | 单 glyph spinner（底层可复用） | 当前未接线，备后续用 |

`RenderedLine` 变体：`UserInput` / `AssistantText` / `ThinkingText` / `ToolSummary` / `Status`。`ThinkingText` 是 c365 新增的灰色思考行。

## 4. 常量

- `TAIL_HEIGHT = 5`（mutable 最大 2 行 + 面板 3 行）
- 面板始终固定 3 行（idle 不填满）
- 面板上方行 = 透明终端底色（正文区）

## 5. app 状态（TuiApp）

- `thinking_phase: bool` — 首个 TextDelta 前为 true
- `thinking_buf: StreamBuffer` — 思考内容流式
- `stream_buf: StreamBuffer` — 主回复流式
- `tool_status: Option<String>` — 工具执行状态标
- `pending_tail() -> Option<(&str, MutableKind)>` — thinking 阶段无内容时返回 `("Thinking…", Thinking)` 占位
- `MutableKind` enum：`Thinking` / `Text` / `Tool` — Tail 据此选 style

## 6. 边界分离

- `handle_xy_event` 返回 `Vec<RenderedLine>`（全是 finalized 行），mod.rs 无条件 `commit_to_scrollback`
- mutable 不进返回值，由 `Tail` 每帧读 `app.pending_tail()` 渲染
- 渲染层 widget 只消费 UI 数据类型，不 match `XyEvent`（spec tui42）
- 每个 widget 有独立 TestBackend 测试（spec tui41）

## 7. 不做的事

- ❌ markdown 增量解析、table holdback、动画线程（纯文本场景不需要）
- ❌ 多行编辑器（后续独立变更）
- ❌ 动态 viewport 高度
- ❌ escape 直写终端（已弃，buffer 路线）

## 8. 已知问题（延迟修复）

工具调用（如 bash）完成后 REPL 退出不继续 loop → `future.md`，需用 `FakeModel` 定位 drain/TurnEnd 路径问题。
