---
depends_on:
  - c1895-add-agent-status-bar-subsystem
---

# `statusline_refresh` 按需工具（仅 tool result）
> **一句话**：statusline_refresh 按需工具（仅 tool result），补充每轮自动尾插的刷新通道；依赖 c1895


> **已升格（2026-08-10）**：当前排序 **#9**。低 ROI，执行前先确认状态栏主链路已落地。
> **来源**：深挖 [`c1895`](../c1895-add-agent-status-bar-subsystem/proposal.md) Q6′。
> **书边界**：栏仍由 Harness 代码维护；工具不得变成「LLM 扫历史写权威栏」。
> **工程约定**：code-first；未切 Designed 前禁止假实现进主路径。

## Why

自动缝（每 generate 尾插）已覆盖「下一轮思考前瞥一眼」的主需求。仍可能后置需要：

- 催 **贵读数**（深 git 概览等）在默认 profile 预算外按需重算；
- 模型在特殊策略下显式要一帧 KV（调试 / 弱模型）。

若工具再 **append 进 transcript**，会与自动尾插双写，加重 `c1897`。故后置工具应 **像普通默认 tool**：只回 tool result。

## What Changes（意向）

- 内置工具名倾向：`statusline_refresh`（可最终按命名规范微调）
- 行为：按当前 profile（或参数指定 provider 子集）**代码重算** → 结构化 KV 写入 **tool result only**
- **MUST NOT** 再向 transcript 盲目尾插 status message（权威栏仍只走 `c1895` 自动缝）
- 可选：参数 `providers: [...]` / `include_expensive: true` 控制贵读数
- 验证：调用后 transcript status 条数不变；tool result 含预期键；与自动尾插并存不双写

## Explicitly deferred / 非目标

- 工具结果是否镜像进 Agent 列（`c1896`）
- 与 `c1897` compact 的交互（本工具不写栏则通常无关）
- TUI 按钮触发 refresh（产品后置）

## Capabilities（意向）

- `agent-tools` / status-bar 工具面

## Out of scope

- Runtime 自动注入缝与默认 profile（→ `c1895`）
- Agent 列 TODO（→ `c1896`）

## Parallel / depends

- **硬依赖**：`c1895`
- 软相关：贵读数 provider 实现进度

## Open Questions

- 默认是否对模型暴露该工具，还是仅 debug / 高档 profile？
- 贵读数白名单与预算是否与 Runtime 单条预算共用常量？

## Ethics

- risk_level: medium（高信任读数进 tool result）
- prohibited_actions: 用工具结果冒充「扫历史总结」；工具副作用写权威栏
- required_evidence: 不双写 transcript 的单测
- refusal_contract: 不宣称按需 refresh 必然提升正确率
- escalation_policy: 默认对全用户暴露前须产品确认

## Provenance

| 字段 | 值 |
|---|---|
| sourced_from | `c1895-add-agent-status-bar-subsystem`（深挖 Q6′：自动每 generate；本波无工具） |
| captured | 2026-08-05 |
| status | purpose-draft |
| intended_semantics | tool result only；name ≈ `statusline_refresh` |
