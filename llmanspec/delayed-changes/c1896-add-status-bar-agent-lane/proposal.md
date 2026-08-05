---
depends_on:
  - c1895-add-agent-status-bar-subsystem
apply_band: P9-deferred
---

# StatusBar · Agent 列（可扩展通道；TODO/计划后挂）

> **⚠️ deferred（2026-08-05）**：随 `c1895` 族移入 `delayed-changes/`。**Todo 产品意向改由 [`c1955`](../../changes/c1955-add-agent-todo-subsystem/proposal.md)**；本文件仅保留「Agent 列 typed 尾插通道」工程想法，升格时再与 Todo 投影对齐。
> **来源**：深挖 [`c1895`](../c1895-add-agent-status-bar-subsystem/proposal.md) Q1。
> **书边界**：栏读数仍须代码/结构化权威；**禁止** LLM 批量扫 transcript 生成权威栏。Agent 列 = 经工具尾插的 **typed message**，不是散文「自己写状态」。
> **工程约定**：code-first defaults；本草案阶段 **不**加 YAML；业务 TODO UX 未定前禁止假实现。

## Why

`c1895` 交付 **Lane Runtime**（Harness 观测 + 注入策略）时，需要同时把 **Lane Agent** 的扩展契约想清楚并落成可挂接口，否则后续「即时计划 / TODO 变状态栏尾插」会挤进 Runtime 列或散落改 Assembler。

本 change 的目的是：**提前占位 + 可扩展通道**，避免忘却；完整 TODO/计划产品形态等业务想清楚后再填。

## What Changes（意向；待业务钉形后切 Designed）

- **AgentLane API（扩展点）**：至少
  - `append(typed_message)`：尾插一条 schema 校验过的消息（只增为默认）
  - `list` / `clear`（或 epoch 界，与 `c1920` 对齐时再钉）
  - 注册表式 **message kind**（`kind: todo | plan | note | …`），新业务 = 新 kind + 渲染器，不改注入主干
- **工具面**：内置工具（如 `status_bar_publish` / 未来 `todo_*`）只写 Agent 列；不直写 Runtime 观测 KV
- **与 Runtime 列边界**：Assembler 合并顺序钉死（建议：轨迹 → Runtime 条 → Agent 条，或文档化可测顺序）；两列不得互相冒充
- **持久化**：是否进 transcript / 是否参与压缩 —— 跟随 `c1930` / `c1895` 已钉的投影契约，本 change 不另发明第二套
- **验证缝**：假 kind 注册 → append → 请求投影可见；非法 payload 拒绝；不依赖真实 TODO UX

## Explicitly deferred（业务未想好）

- TODO 条目生命周期、TUI 展示、与用户可见 checklist 是否同源
- `rewrite_todo_list` vs `update_todo_status`（书实验 2-8）是否原样搬、还是 xylitol 自有形状
- 即时计划是否独立 kind、是否可被 Runtime「进度摘要」只读镜像

## Capabilities（意向）

- `agent-*`（status-bar agent lane / channel）
- 可选后置：`app-tui-*` 仅当要有用户面 checklist

## Impact

- `c1895` apply 时即可预留 trait / 空实现，本 change 再填 kind 与工具
- 降低「TODO 直接塞进 system / Runtime KV」的临时债

## Out of scope

- Runtime 列读数与 `off|replace|append`（→ `c1895`）
- tool_search / MCP（→ `c1900`）
- 完整 TODO 产品与 TUI

## Parallel / depends

- **硬依赖**：`c1895`（Runtime 列与注入缝先立）
- 软相关：`c1930`（投影/持久）、`c1920`（epoch）

## Open Questions

- Agent 列默认 append-only 是否允许「同 id 覆盖」类 replace（仅 kind 内），还是一律新条？
- 工具名与权限：是否算高信任写面（与状态栏投毒同一 ethics）？
- 业务 TODO 形态（用户钉后再切 Designed）

## Ethics

- risk_level: medium（模型高度信任末尾 meta）
- prohibited_actions: LLM 扫历史写权威栏；未校验散文直接尾插；把外部不可信全文当 kind payload
- required_evidence: kind 注册 + schema 拒绝路径可测
- refusal_contract: 不宣称 TODO 状态栏必然提升完成率
- escalation_policy: 默认开启 Agent 列工具前须产品确认

## Provenance

| 字段 | 值 |
|---|---|
| sourced_from | `c1895-add-agent-status-bar-subsystem`（深挖 Q1：双列；本波只通 Runtime） |
| captured | 2026-08-05 |
| status | purpose-draft（想法保险箱；非 readyToImplement） |
