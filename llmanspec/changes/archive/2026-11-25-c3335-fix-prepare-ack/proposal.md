---
depends_on: []
title: 新建、分叉和恢复会话时准备确认只有一份
needs_specs_change: false
branch: sdd/c3335-fix-prepare-ack
base_branch: main
base_sha: ee0bfce5f943e6372b2943a4f1bca2e1b9148bb6
---

# 新建、分叉和恢复会话时准备确认只有一份

## Why

`agent-runtime` 的 r1052 要求新建会话、分叉会话和恢复会话清掉工具冻结、重置准备确认，并重新开火准备。不得沿用上一会话的确认去跳过 MCP 首轮门禁。

驱动上的 `prepare_acked` 和 actor 探针上的 `prepare_acked` 是两份副本。`actor.rs` 的 new、fork、resume 只把探针写成 false。驱动字段保持原值。

## What Changes

准备确认只留 actor 探针上的那一份。驱动读取它，不再自留一份。

`fire_session_prepare`、`settle_tool_gate`、`ensure_tool_table_frozen`、`poll_prepare_progress`、`arm_tool_freeze_gate` 和 actor 的 new、fork、resume 都走这一个写入点。

本 change 不折叠其余准备字段，不去重 `NoActiveSession` 守卫，也不改工具定稿政策。

## Capabilities

- `agent-runtime`：r1052 已写明重置准备确认。本 change 让实现符合该规则，不改规则文本。

## Impact

- 代码：`crates/xylitol-runtime/src/driver/in_process/mod.rs`、`actor.rs`、`prepare.rs`，以及该 crate 内覆盖 new、fork、resume 的单测。
- 验证：单测断言这三条路径之后，门禁读取和资源快照读取看到同一份 false。既有 MCP 门禁单测保持通过。不新增 BDD 场景。r1052 写明由单测覆盖。
- 与 `c3340-refactor-session-slot`、`c3345-refactor-ui-model-writes` 无文件重叠，无 `depends_on`。建议先落地本 change，因为规则今天就会破。

## Further Notes

模型与最短反例在 `research/model.md`。模型不含 hub 的 `view.complete`，也不含其余准备字段。
