---
depends_on: []
branch: sdd/c1955-add-agent-todo-subsystem
base_sha: 839990764660a6b9f9014d1a88ee2d35aafeccd6
checkpointed: false
---

# Agent Todo 子系统（TUI + 工具 + 内部流转）
> **一句话**：会话内持久 Todo SSOT：TUI checklist + LLM 工具 + resume 共享，治目标漂移/漏子任务；当前产品优先线
> **当前排序**：#1（2026-08-10 自 delayed-changes 升格入 active）
> **阶段**：Designed / pre-start（见 `design.md` / `tasks.md`；**未** `change start`）

> **产品优先于状态栏**：先立 Todo 产品 SSOT（排序 #1），状态栏族（排序 #5–9）仅作可选投影。
> **书指针**：Ch2 实验 2-8「TODO 列表管理」（`rewrite_todo_list` / `update_todo_status`）；Ch5 coding 任务拆解。书语不进 live specs。
> **工程约定**：code-first defaults；不定 YAML；TUI 与内部状态 MUST 同源（见 `docs/roadmaps/Web与TUI同源.md` 精神）。

## Why

个人 coding agent 的常见失败是 **目标漂移 / 漏子任务 / 弱模型空转**，不是缺每轮时钟。需要：

- 用户可见的 checklist（TUI）
- LLM 可调用的查看/更新工具
- 会话内持久的 Todo SSOT（供 resume / 日后子 agent 经同 session store 共享登记，而非「对齐父状态栏」）

状态栏若存在，只从本 SSOT **渲染一帧**（完成/未完成摘要）；**禁止**栏 auto 摘要当 Todo 真源。高时效事实走 tool result / 事件，不靠过时栏优化。

## What Changes

- **领域**：`TodoItem { id, content, status }`；`status ∈ pending|in_progress|completed|cancelled`；至多一条 `in_progress`（详见 `design.md` D9）
- **持久**：`SessionEntry::Custom`，`custom_type = "agent_todo"`，latest-wins 全量快照；不进 LLM 前缀（D5/D8）
- **工具**：`todo_list` / `todo_rewrite` / `todo_update` 进 `default_tools`（D6/D10；产品优先线已确认默认入表）
- **TUI**：可折叠对话条目 checklist；默认一行摘要；禁常驻 Plan 侧栏（D1）
- **边界**：与 `c1895`/`c1896` 只读投影；本波不实现栏（D7）

## Capabilities（Specs landing 意向）

- **新** `agent-todo`
- 修订 `agent-tools`（内置闭集）
- 按需 `agent-session-store` / `app-tui-transcript`（或 `app-tui-todo`）

## Out of scope

- 完整 Agent 状态栏 always-on（→ delayed `c1895`）
- Agent 列 typed 通道业务填空（→ `c1896`）
- tool_search / MCP
- 自动用 LLM 扫轨迹生成 Todo
- max-attempt / 熔断（D3）
- 即时计划 plan 模型（D2）
- 子 agent 实现（D4 仅钉共享意向）
- 用户手改 checklist / `/todo` slash

## Parallel / depends

- **硬依赖**：无（可先于状态栏）
- **软相关**：`c1930`（投影契约：Todo 故意不进 provider input）；升格后的 `c1895`/`c1896` 仅作可选投影消费者
- **替代关系**：原 `c1896` 的 Todo 业务意向 **并入本 change**；通道若仍需要，随状态栏升格再拆

## Open Questions（已钉 → design D1–D10）

| # | 原问 | 决议 |
|---|---|---|
| Q1 | TUI 默认展开 vs 折叠；是否占主 transcript | **D1**：可折叠对话条目；默认一行摘要；禁侧栏 / 禁 status 主清单 |
| Q2 | 与即时计划是否同一模型 | **D2**：本波仅 Todo；plan 另案 |
| Q3 | max-attempt / 熔断归属 | **D3**：本波不做 |
| Q4 | 子 agent 共享 vs 命名空间 | **D4**：单会话一份；同 store 共享；本波不实现子 agent |
| — | （设计增补）持久 / 工具 / 栏边界 / 冻表 | **D5–D10** 见 `design.md` |

无未决 Open Question；Start 前无需再问产品。

## Ethics

- risk_level: medium（用户可见任务状态）
- prohibited_actions: 静默丢用户 Todo；用状态栏冒充 Todo SSOT；LLM 批量编造 Todo 当权威
- required_evidence: 工具更新 ↔ TUI ↔ 持久 三端一致可测
- refusal_contract: 不宣称 Todo 必然提升完成率
- escalation_policy: ~~默认强推 Todo 工具进全量 tools 表前须产品确认~~ → **已确认**（优先线 #1 / D6）

## Provenance

| 字段 | 值 |
|---|---|
| replaces_intent_of | delayed `c1896` Todo 业务部分 |
| deferred_alongside | `c1895`–`c1898`（状态栏族 → `llmanspec/delayed-changes/next-todo/`） |
| captured | 2026-08-05 |
| designed | 2026-08-12 |
| status | designed / pre-start（未 branch） |
