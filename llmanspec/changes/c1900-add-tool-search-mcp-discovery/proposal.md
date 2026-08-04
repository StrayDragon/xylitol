---
depends_on:
  - c1880-update-responses-first-api-boundary
  - c1890-add-responses-context-policy-assembler
---

# tool_search + MCP 内部目录（对齐 Codex：不热改 tools[]）

> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md) §4（术语对照 §7）
> **对照实现**：姊妹仓 `../codex`（`ToolExposure::Deferred` + `tool_search_call`/`tool_search_output` + `defer_loading`）；动态工具进**轨迹 item**，不进顶栏 `tools[]`。
> **书指针**：《深入理解 AI Agent》Ch2 工具只增不改披露；书语仅经 research §7 映射，**禁止**写入 live specs。
> **自包含**：Search 档对齐 Codex 主路径；Full 档保留逃生（可继续 next-turn 热并整表）。**不**依赖 `c1920` epoch（已 deferred）。
> **view 契约**：search 注入的持久化/投影标记遵守 [`c1930`](../c1930-update-session-provider-view-contract/proposal.md)（实现时对齐）。
> **工程约定（本波次）**：策略默认 **code-first**：`defaults.rs` 纯常量；**不**新增 YAML 旋钮；**不**用 env 当未暴露配置面。用户面 YAML 仅既有字段（如 `api`）。真源见 [`c1880`](../archive/2026-08-04-c1880-update-responses-first-api-boundary/proposal.md)。
> **兼容验证**：形似 Responses 的端（DeepSeek、**llama.cpp**、本地 Ornith 等）对 `tool_search_*` / `defer_loading` **未必**一致；MUST 经 WirePolicy 显式声明，禁止仅因 `api=openai-responses` 假定官方行为。本 change 先对齐 Codex/官方形状 + 单测/假 provider；**llama.cpp + Ornith 真机验证列为后续门槛**（可挂 live provider 测或手工清单）。

## Why

现状：MCP 异步 settle 后热合并进 `ToolSet`，每轮完整 schema 进 Responses `tools[]` → 改稳定前缀，且 MCP 一多伤害选择与 token。

Codex（OpenAI Responses）路径更简单：

1. 异步 MCP **照旧**（不挡 TTI）。
2. MCP 工具 **Deferred**：只进内部 registry / 可搜目录，**不**进请求顶栏 `tools[]`。
3. 顶栏 = 稳定 Direct 子集 + 元工具 `tool_search`（跨轮名表宜稳定；MCP source 变化最多改 search **description**，不把 MCP schema 塞进顶栏）。
4. 模型 `tool_search_call` → 客户端回 `tool_search_output`（轨迹 item，内含 function/namespace schema 且 `defer_loading: true`）。
5. 之后模型可直接 function_call；运行时从 registry dispatch。

「阻塞输入直到 MCP ready」不作为主方案。

## What Changes

- ContextPolicy `tools_mode`：`search`（主交付）与显式 `full`（逃生/兼容）。
- **search 档**：
  - provider `tools[]` **不**因 MCP settle 自动变长；Direct 顺序自首条起保持稳定（对齐 Codex prompt-cache 测法心智）。
  - 内置元工具 `tool_search`（client-executed 为主；execution 字段对齐 Codex `client`）。
  - 发现逻辑 BM25/等价查内部 registry（已武装 MCP + 可 Deferred 的扩展工具）；搜**触发时最新**条目。
  - 主轨迹吸收 `tool_search_output`（或 wire 等价），schema 只增不改、固定首次位置，后续轮不删、不搬到末尾。
  - **禁止**把「搜到的工具」再 append 进顶栏 `tools[]`（那是 Codex 刻意不做的捷径）。
- **full 档**：可保留现有「MCP settle → next turn 注入 tools[]」行为（接受前缀变化）。
- WirePolicy / capabilities：声明是否支持 hosted `tool_search`、`tool_search_output`、`defer_loading`；不支持则 client 伪造等价 input item 或降级 `full`。
- `/reload`：更新内部 registry；search 档下顶栏 `tools[]` 不因 MCP 目录变化而变长（description 是否刷新钉在 design）。
- 文档：Codex 对照要点；兼容端差异；Ornith/llama.cpp 验证清单指针。

## Capabilities（意向）

- `agent-*` / 工具系统
- `infra` MCP 装配与 registry
- `package-ai-bridge`（Responses `tool_search_*` / `defer_loading` 形状）
- 产品：`扩展能力-MCP` / 工具与权限

## Impact

- 与 Codex 同构：搜最新 + 顶栏稳 + 动态定义在轨迹里。
- 兼容端需显式测；不能假设「挂了 openai-responses 标签就等于 OpenAI」。

## Out of scope

- `c1920` context epoch（deferred；本路径不需要）
- 阻塞输入直到 MCP ready 的完整 TUI 协议
- resume 历史差分重放全量 tools 表
- 状态栏（→ delayed `c1895`）
- Anthropic Tool Search 原生块
- 本 change 内完成 llama.cpp/Ornith 真机通过（列为后续验证门槛，不挡 Codex 对齐落地）

## Parallel / depends

- **硬依赖**：`c1880`（WirePolicy/capabilities）、`c1890`（Assembler/Policy）
- **软配合**：`c1930`（view 标记）；`c1925` 可并行
- **不依赖**：delayed `c1920`

## Open Questions

- 开箱默认 `search` vs `full`（弱模型不会搜时的降级）
- MCP source 变化时是否刷新 `tool_search` description（Codex 会；会轻微动顶栏字节）
- 兼容端无 `tool_search_output` 时：伪造 item vs 强制 `full` 的产品默认

## Ethics

- risk_level: medium
- prohibited_actions: search 档下静默热扩 provider `tools[]`；因 `api=openai-responses` 假定 hosted tool_search 可用；把未声明兼容的端标成与 OpenAI 等价
- required_evidence: settle 后 registry 有工具但顶栏 tools 长度稳定；经 `tool_search_output` 可加载并调用；WirePolicy 对「无 tool_search」端有明确降级；假 provider/单测覆盖 Codex 形状
- refusal_contract: 不把「阻塞至全加载」写成 MUST；不把未测的 llama.cpp/Ornith 写成已支持
- escalation_policy: 开箱强制 search 若伤弱模型，须可切 `full`；Ornith/llama.cpp 真机结果须回写 WirePolicy/文档后再声称兼容
