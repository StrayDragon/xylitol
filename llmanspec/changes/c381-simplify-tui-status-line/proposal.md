---
change_id: c381-simplify-tui-status-line
title: 简化 TUI 状态行为 spinner + 单活动标签（去掉 Turn 轮数 / 模型名）
status: proposed
priority: 381
depends_on:
  - c380-add-tui-status-line
author: agent
---

# c381-simplify-tui-status-line

## Why

c380 落地的状态行是三段式（spinner+活动标签 / Turn 轮数 / 模型名）。真终端测试后用户（2026-07-02）反馈：**只需要 spinner + Working**，其余去掉。

另发现：**模型名从不动**——因为 `XyEvent::ModelSelect` 在 agent 层从不发出（grep 全仓库无 yield/emit ModelSelect，只有消费方）。`model_name` 字段永远是 None，right 段一直空。与其让一个永不填充的段占位，不如直接去掉。

## What Changes

1. **`StatusLine` widget 简化**：去掉三段布局（left/center/right），改为单段左对齐——spinner + 活动标签。
2. **`StatusSegments` 简化**：去掉 `center` / `right` 字段，只留 `streaming` + `left_label`。
3. **`TuiApp` 删字段**：`turn_index` / `model_name`（无数据源 / 非必需）。
4. **`handle_xy_event` 回退**：TurnStart / ModelSelect 不再记状态（回到 c380 前的 no-op）。

## Capabilities

- `app-tui`（修改）：修订 tui53——状态行从三段简化为 spinner + 单标签。

## Impact

- **受影响代码**：`components/status_line.rs`、`app.rs`（字段 + status_segments + 事件处理）。
- **风险**：低。纯 UI 简化，不动事件循环 / 布局结构（StatusLine 仍固定 1 行，TAIL_HEIGHT 不变）。

## 反降级护栏

- [ ] StatusLine MUST 显示 spinner + 活动标签（Working / Running {tool} / Ready），MUST NOT 再显示 Turn 轮数或模型名。
- [ ] streaming 时 spinner 接线不变（Tick 推进）。
- [ ] idle 显示 Ready。
