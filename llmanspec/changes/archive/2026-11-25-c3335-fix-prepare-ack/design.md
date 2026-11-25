# Design: c3335-fix-prepare-ack

## 目标

新建、分叉和恢复会话之后，门禁读取和资源快照读取看到同一份准备确认。

## 两种形状

| | 形状 A | 形状 B |
|---|---|---|
| 存放 | 只留 actor 探针上的 `prepare_acked` | 只留驱动字段，探针删掉 |
| 读取 | `mcp_blocks_agent`、`poll_prepare_progress`、`snapshot_from_view` 都读探针 | 这三个读取都改读驱动字段 |
| 写入 | `fire_session_prepare`、`settle_tool_gate`、`ensure_tool_table_frozen`、`poll_prepare_progress`、`arm_tool_freeze_gate`，以及 actor 的 new、fork、resume，都走 `set_prepare_acked` | new、fork、resume 改为写驱动字段 |

## 选择

用形状 A。

探针已经是 `mcp_blocks_agent` 和 `poll_prepare_progress` 的读取点。驱动字段是多出来的那一份，new、fork、resume 碰不到它。删掉驱动上的 `prepare_acked`。`snapshot_from_view` 改为读探针：`view.complete || (tools_table_frozen && probe)`。

其余准备字段、`NoActiveSession` 守卫和工具定稿政策不动。

## 完成时的读法

new、fork、resume 各自跑完后，门禁读取和 `snapshot_from_view` 都看到 false。既有 MCP 门禁单测保持通过。
