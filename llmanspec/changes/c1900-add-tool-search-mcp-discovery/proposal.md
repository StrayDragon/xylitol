---
depends_on:
  - c1880-update-responses-first-api-boundary
  - c1890-add-responses-context-policy-assembler
---

# tool_search + MCP 内部目录（不热改 provider tools 表）

> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md) §4；书 Ch2/Ch4；OpenAI tool_search / defer_loading。
> **波次**：Wave C（依赖 `c1880`+`c1890`；可与 `c1895`/`c1905`/`c1910` 并行）
> **自包含**：交付 MCP 披露主路径；明确**不**以「阻塞输入直到 MCP 全加载」为主方案。

## Why

现状：MCP 异步 settle 后热合并进 `ToolSet`，每轮完整 schema 进 Responses `tools` → 改稳定前缀，且 MCP 一多伤害选择与 token。

更简单且与 cache/注意力更合拍的路径：

1. 异步加载**照旧**（不挡 TTI）。
2. 热合并只进**内部 registry**。
3. 发给 provider 的 `tools` = 稳定子集（核心工具 + `tool_search` 元工具等）。
4. 模型经 search 发现后，结果以「只增不改」进入主轨迹（固定首次位置）。

备选「禁止用户输入直到全部 MCP ready」在 resume / 历史已含中途工具痕迹时要做差分重放，复杂度高——**本 change 不采用为主路径**。

## What Changes

- ContextPolicy `tools` 暴露档：至少 `search`（本 change 主交付）与显式 `full`（逃生/兼容）。
- **search 档**：
  - provider `tools` **不**因 MCP settle 自动变长；
  - 内置元工具 `tool_search`（或 client-executed 等价名）；
  - 发现逻辑查内部 registry（核心 + 已武装 MCP）；
  - 发现请求：默认**当前用户 model 另开短请求**；配置可 `sidecar_model`；
  - 主轨迹只吸收 search 结果（schema/引用），按 Responses/兼容约定固定位置，后续轮不删除、不搬到最新末尾。
- capabilities：无 hosted `tool_search` 的兼容端走 client-executed；由 **flavor + 配置**声明（接 `c1880`），**禁止**因 `api=openai-responses` 就假定官方 `tool_search` / `defer_loading` 可用（DeepSeek 等形似端常见差异）。
- `/reload`：更新内部 registry；provider 稳定 `tools` 表不变（除非用户显式切 `full` 或开新「工具世代」——世代语义本 change 写清最小集）。
- 文档：相对「阻塞至全加载」的复杂度对比；推荐 search；并写明 flavor 覆盖入口。

## Capabilities（意向）

- `agent-*` / 工具系统
- `infra` MCP 装配与 registry
- `package-ai-bridge`（Responses tools / tool_search_output 形状）
- 产品：`扩展能力-MCP` / 工具与权限

## Impact

- 启动不挡 TTI 与 cache 友好可兼得。
- 主上下文不被 MCP 全量 schema 淹没。

## Out of scope

- 阻塞输入直到 MCP ready 的完整 TUI 协议（刻意不做主路径）
- resume 历史差分重放全量 tools 表（随「不做阻塞方案」一起放下）
- 状态栏（→ `c1895`）
- Anthropic Tool Search 原生块

## Parallel / depends

- **硬依赖**：`c1880`（capabilities）、`c1890`（Assembler/Policy 钩子）
- 与 `c1895`/`c1905`/`c1910` 同波可并行

## Open Questions

- 元工具对外名：`tool_search` vs `discover_tools`（兼容端）
- `full` 档是否仅调试/配置显式，开箱 MCP 是否默认 `search`

## Ethics

- risk_level: medium
- prohibited_actions: search 档下静默热扩 provider `tools`；把 sidecar 整段思考并进主轨迹
- required_evidence: settle 后 registry 有工具但 provider tools 表长度稳定；search 可加载并调用
- refusal_contract: 不把「阻塞至全加载」写成 MUST
- escalation_policy: 若开箱强制 search 导致弱模型不会搜，须可切 `full`
