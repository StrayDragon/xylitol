---
depends_on: []
---

# Agent Todo 子系统（TUI + 工具 + 内部流转）
> **一句话**：会话内持久 Todo SSOT：TUI checklist + LLM 工具 + resume 共享，治目标漂移/漏子任务；当前产品优先线
> **当前排序**：#1（2026-08-10 自 delayed-changes 升格入 active）


> **已升格（2026-08-10）**：自 delayed-changes 移入 active 待处理队列，当前排序 **#1**。


> **产品优先于状态栏**：先立 Todo 产品 SSOT（排序 #1），状态栏族（排序 #5–9）仅作可选投影。
> **书指针**：Ch2 实验 2-8「TODO 列表管理」（`rewrite_todo_list` / `update_todo_status`）；Ch5 coding 任务拆解。书语不进 live specs。
> **工程约定**：code-first defaults；本草案阶段不定 YAML；TUI 与内部状态 MUST 同源（见 `docs/roadmaps/Web与TUI同源.md` 精神）。

## Why

个人 coding agent 的常见失败是 **目标漂移 / 漏子任务 / 弱模型空转**，不是缺每轮时钟。需要：

- 用户可见的 checklist（TUI）
- LLM 可调用的查看/更新工具
- 会话内持久的 Todo SSOT（供 resume / 子 agent 共享登记，而非「对齐父状态栏」）

状态栏若存在，只从本 SSOT **渲染一帧**（完成/未完成摘要）；**禁止**栏 auto 摘要当 Todo 真源。高时效事实走 tool result / 事件，不靠过时栏优化。

## What Changes（意向）

- Todo 领域模型：item id、内容、状态（pending/in_progress/completed/cancelled 等，最终枚举 design 钉）、可选尝试上限 / 关联工具调用
- 持久化进 session（与 `c1930` 投影契约对齐时再钉标记）
- 内置工具：至少 list/get + update（形状可参考书，不必照搬函数名）
- TUI：展示当前 Todo（chrome 或面板；词汇表另对齐）
- 可选后置：达 max-attempt 时登记事件；子 agent 读写同一 Todo 总线

## Out of scope（本草案）

- 完整 Agent 状态栏 always-on（→ delayed `c1895`）
- tool_search / MCP
- 自动用 LLM 扫轨迹生成 Todo

## Parallel / depends

- **硬依赖**：无（可先于状态栏）
- **软相关**：`c1930`（持久/投影）；升格后的 `c1895`/`c1896` 仅作可选投影消费者
- **替代关系**：原 `c1896`（Agent 列通道）的 Todo 业务意向 **并入本 change**；通道若仍需要，随状态栏升格再拆

## Open Questions

- TUI 默认展开 vs 折叠；是否占用主 transcript 行
- 与即时计划（plan）是否同一模型还是两种 kind
- max-attempt / 熔断是 Todo 字段还是独立 harness 闸
- 子 agent：共享 store vs 命名空间隔离

## Ethics

- risk_level: medium（用户可见任务状态）
- prohibited_actions: 静默丢用户 Todo；用状态栏冒充 Todo SSOT；LLM 批量编造 Todo 当权威
- required_evidence: 工具更新 ↔ TUI ↔ 持久 三端一致可测
- refusal_contract: 不宣称 Todo 必然提升完成率
- escalation_policy: 默认强推 Todo 工具进全量 tools 表前须产品确认

## Provenance

| 字段 | 值 |
|---|---|
| replaces_intent_of | delayed `c1896` Todo 业务部分 |
| deferred_alongside | `c1895`–`c1898`（状态栏族 → `llmanspec/delayed-changes/`） |
| captured | 2026-08-05 |
| status | purpose-draft |
