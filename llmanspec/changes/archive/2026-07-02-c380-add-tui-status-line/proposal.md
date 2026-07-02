---
change_id: c380-add-tui-status-line
title: TUI 加固定状态行（spinner + 工具名 + ReAct 轮数 + 模型名）
status: proposed
priority: 380
depends_on:
  - c375-fix-react-loop-tool-continuation
author: agent
---

# c380-add-tui-status-line

## Why

c365/c370 落地后 TUI 渲染就绪，但**缺一个固定的工作状态指示**：用户无法直观看到「agent 是否在工作、在跑什么工具、第几轮、用什么模型」。现有 `Spinner` widget（6415f48）已就位但未接线；`TuiApp` 已有 `spinner_idx`/`tool_status` 但未在 UI 呈现。用户（2026-07-02）明确需要一个**始终占位**的独立状态行，位于流式输出行下方、输入框上方。

## What Changes

1. **新增 `StatusLine` widget**（`components/status_line.rs`，全新实现，不同于 c365 删除的旧 `status_line`）：始终 1 行，**三段式布局**（left 左对齐 / center 居中 / right 右对齐）：
   - **left**：spinner（固定 left 段首位）+ 活动标签（`Working…` / `Running {tool}` / `Ready`）
   - **center**：`Turn {n}`（streaming 时）
   - **right**：`{model_name}`
   - 三段是可扩展骨架——未来加状态项（token 计数/耗时/上下文占用）只往对应段加内容，不改 widget 渲染逻辑（对标 vim airline / VSCode status bar）。
2. **`TuiApp` 加状态字段 + `status_segments()` 访问器**：`turn_index: Option<u32>`（TurnStart 设）、`model_name: Option<String>`（ModelSelect 设）。StatusLine 数据驱动——消费 `status_segments()`，widget 不自己 match 状态。
3. **`handle_xy_event` 处理新事件**：`TurnStart` → 记 turn_index；`ModelSelect` → 记 model_name。
4. **Tail 布局改三段**：`MutableLine（顶，透明）→ StatusLine（中，固定 1 行）→ BottomPanel（底，3 行）`。StatusLine 始终占位。
5. **`TAIL_HEIGHT` 5 → 6**（+1 给 StatusLine）。
6. **Spinner 接线**：StatusLine 在 streaming 时消费 `app.spinner_idx()` 渲染 `Spinner` widget（tick 已驱动 spinner_idx 推进）。
7. **统一 SPINNER 常量**：app.rs 与 spinner.rs 各有一份 `SPINNER`，改为 spinner.rs 单一来源，app.rs 引用之。

## Capabilities

- `app-tui`（修改）：新增固定状态行 spec。

## Impact

- **受影响代码**：新增 `components/status_line.rs`；`app.rs`（字段 + 事件处理 + SPINNER 引用）；`tail.rs`（三段布局）；`terminal.rs`（TAIL_HEIGHT）；`components/mod.rs`（导出）；`theme.rs`（可能加 status 样式 token）。
- **受影响规范**：`app-tui`。
- **风险**：低-中。TAIL_HEIGHT 变化 + Tail 布局重构，需更新既有 Tail harness 测试的行号断言。Spinner 接线是纯增量。

## 反降级护栏

- [ ] StatusLine MUST 始终占位（idle 也在，显示 model_name + Ready）。
- [ ] StatusLine MUST 三段式布局（left/center/right），可扩展——未来加状态项不改 widget 渲染逻辑。
- [ ] spinner MUST 固定在 left 段首位（streaming 时），动画由现有 Tick 推进。
- [ ] StatusLine MUST 数据驱动：消费 `status_segments()` 访问器，widget 不自己 match 业务状态。
- [ ] streaming 时 left 显示 Working/工具名，center 显示 Turn 轮数，right 显示模型名。
- [ ] Spinner widget MUST 实际接线（非仅存在）——否则又是未驱动的骨架。
- [ ] SPINNER 常量 MUST 单一来源（spinner.rs），app.rs 引用，不重复定义。
- [ ] 既有 Tail harness 测试 MUST 更新并通过（布局行号变化）。
- [ ] TAIL_HEIGHT 调整 MUST 与新三段布局总和一致（mutable 可变 + status 1 + panel 3）。
