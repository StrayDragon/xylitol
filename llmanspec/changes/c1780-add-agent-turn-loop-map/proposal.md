---
depends_on: []
status: purpose-draft
---

## Why

`/codebase-life-review`（3a）对 **agent-core · 脉络 A「一轮对话主循环」** 做了地图级走查：入口 seam、主干时序、易迷路点已对齐代码事实。临时笔记在 `_HANDOFF` 不宜当 SSOT；把结论收成 purpose-draft，便于后续正式化（游记级事件清单、文档补强、或模板拆页），并删除易腐 HANDOFF。

## What Changes

本草案**不改运行时行为**；记录已确认地图与开放意向：

- 定调 / 分层抽查（2026-07-30）仍成立：`XyDriver` + `XyEvent` 主线；`agent` ↛ `infra`（生产）；面经 `app/core`
- 脉络 A 入口索引与主干时序（`run` / steer / follow_up / abort → ReAct outer → 意图收齐再工具批）
- 易迷路点：面不持有队列；QueueUpdate 双路径；steer vs follow_up 时机；EventSink ≠ turn 总线
- **后置意向**（未承诺本 change 交付）：游记级 `XyEvent` 名清单；macro/vein 文档拆页；开脉络 B/C/D

## Capabilities

| 意向 | 说明 |
|---|---|
| （文档 / meta）`architecture` 或 `agent-*` 补强 | 仅当 propose 时决定是否升格 `docs/architecture/一轮对话.md` 等 |
| 无运行时 capability | 地图级结论本身不引入 MUST 行为变更 |

## Impact

- **代码**：无（草案阶段）
- **文档**：正式化时可能回写 architecture / 交接图；勿把本 proposal 当运行时合约
- **测试**：无

## Open Questions

- [ ] 地图级是否够用，还是要游记级（首 user → 第一次模型 → 第一批工具的事件名清单）？
- [ ] 人生回顾模板：继续单文件，还是 macro + vein 拆页？
- [ ] 本 draft 正式化时走 docs-only，还是仅归档探索结论后关闭？

## 来源摘要（原 HANDOFF）

- 触发：`/codebase-life-review` · 3a；镜头：整仓 → 核心 agent 流程；深度：地图级
- 入口：`XyDriver` ← TUI `drain_pending`；进程内 `XyInProcessDriver` → `AgentRuntime::run_with_id` / `run_react_loop`
- 成功结束：`'outer` 无 follow-up → 流 EOF / `AgentEnd` → 面 idle
- 张力：测试缝允许 agent 碰 infra；Driver 可调部分 infra vs 面禁止 reach；体量豁免靠脉络图而非先拆文件
