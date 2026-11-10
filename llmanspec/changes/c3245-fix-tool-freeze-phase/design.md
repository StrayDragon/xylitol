# Design — 工具表冻结相位

## 1. 目标与边界

- Gating 只允许作为短暂中间态；能力覆盖 / 重建工具表的流程结束必须回到 Frozen 或显式 Unfrozen。
- Frozen 拒绝写入对调用方可见，不再静默 no-op。
- 边界：不改变 MCP 首轮门禁语义（15s 超时放行 + 迟到工具下一回合再冻结）。

## 2. 决策摘要

| 决策 | 内容 | 状态 |
|---|---|---|
| 转移矩阵 | 见 §3；`reopen` 要求来源（指纹匹配或显式重建指令） | 定稿方向 |
| overlay 序列 | apply_pending_capability_overlay 改 reopen → set_tools → freeze_tools（补终步） | 定稿方向 |
| 写入拒绝 | set_tools 族返回 `ToolsWriteOutcome { Applied, RejectedFrozen, RejectedMidTurn }` | 推荐，待 review |
| Gating 超时 | 对齐 MCP_FIRST_TURN_GATE_TIMEOUT；超时回 Unfrozen 并告警 | 推荐，待 review |

## 3. 转移矩阵

| 当前 \ 事件 | begin_gating | reopen(source) | freeze | clear | apply_done |
|---|---|---|---|---|---|
| Unfrozen | Gating | — | Frozen | — | — |
| Gating | Gating | Gating | Frozen | Unfrozen | Frozen（新：overlay 序列终步） |
| Frozen | （拒绝，已是终态） | Gating | Frozen | Unfrozen | — |

- `begin_tool_gating` 在 Frozen 时从「静默跳过」改为 Outcome::RejectedFrozen（调用方决定是否先 reopen）。
- Gating 超时：guard 任务到期未收口则 `clear` + warn（观测计数），避免长期解冻。

## 4. capability 覆盖相位

```text
CapabilityOverlay { disabled: BTreeSet, applied: BTreeSet, phase: Idle | Pending }
apply_pending(): Pending → reopen → set_tools → freeze → applied = disabled → Idle
```

- 集合差保留为派生断言（debug_assert applied == disabled when Idle），相位由显式字段承载。

## 5. 测试边界（seam）

- 单测：转移矩阵全组合 + overlay 序列结束断言 Frozen。
- BDD：infra-mcp 首轮门禁回归（gate 超时放行、迟到工具再冻结）；agent-runtime mid-turn 工具改写拒绝场景。

## 6. 风险与回滚

- 风险：调用点把静默跳过当依赖（frozen 时 set_tools 被忽略的场景）；改显式 Outcome 后逐调用点确认处理分支。
- 回滚：矩阵集中在 tools_ops 单文件。
