---
depends_on: []
title: 端上 transcript 和忙碌状态只有一处可写
needs_specs_change: false
---

# 端上 transcript 和忙碌状态只有一处可写

## Why

`app-tui-input` 与 `app-tui-transcript` 已经规定队列条和 transcript 的可见行为。`UiModel` 的字段是公开的。bridge、`history_window` 和 host 都能写。

host 的 `recalc_busy` 与 `end_gated_assemble_idle` 直接写 `phase` 和 `status`，不经过 `settle_after_turn_boundary`。队列非空时，这两处和 settle 对状态文字的处理不一样。现有场景没有单独规定这句状态文字，所以本 change 不把它写成新规则。

## What Changes

会话事实条目只由 bridge 的方法追加。历史窗口注解只由 `UiModel` 上的一个方法插入或删除，`history_window` 调用它。`phase` 和 `status` 只由 `UiModel` 上的 settle 和忙碌设置方法修改。host 改为调用这些方法，不再写字段。

已有队列条场景和 AgentEnd 回到空闲的场景保持通过。若某调用点必须保留与 settle 不同的状态文字，它调用一个已有场景覆盖的具名方法，不新增文案。

本 change 不改 `SelectList`、`PaintCache`、`tui.terminal`，也不压缩测试脚手架。

## Capabilities

- `app-tui-input`：队列条与 Host 事件带的既有规则。
- `app-tui-transcript`：transcript 条目的既有规则。
- 本 change 不改这些规则的文本。

## Impact

- 代码：`crates/xylitol-app-tui/src/bridge/model.rs`、`host/mod.rs`、`history_window.rs`，以及 bridge handler 里直接写 `phase` 或 `status` 的位置。`layout/root` 对条目的生产路径保持只读。
- 验证：现有队列条与 AgentEnd 的 harness 切片保持通过。不新增 BDD 场景。
- 与另外两份草案无文件重叠，无 `depends_on`。建议最后落地。它碰的是呈现，不碰已经破掉的准备确认规则。

## Further Notes

这里没有做状态图。产品规格没有把「队列非空时必须显示 Delivering steering」写成一条可判定规则。强行建成状态机，会把 settle 函数的实现选择写成不变量。
