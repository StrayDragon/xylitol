---
depends_on:
  - c1890-add-responses-context-policy-assembler
---

# Session SSOT ↔ Provider view 契约

> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md)（术语对照 §7）
> **书指针**：《深入理解 AI Agent》Ch2「上下文：决定 Agent 能力上限」轨迹 vs 投影（姊妹仓 `ai-agent-book/book/chapter2.md`）；书语仅经 research §7 术语表映射，**禁止**写入 live specs。
> **自包含**：钉「什么进持久会话、什么只进请求投影」；供状态栏 / tool_search / 压缩冻结共用，避免并行实现各写标记。
> **工程约定（本波次）**：策略默认 **code-first**：`defaults.rs` 纯常量（改文件调试）；**不**新增 YAML 旋钮；**不**用 env 当未暴露配置面。用户面 YAML 仅既有字段（如 `api`）。真源见 [`c1880`](../archive/2026-08-04-c1880-update-responses-first-api-boundary/proposal.md)。

## Why

状态栏 meta、tool_search 注入、压缩替换串、Env 折叠消息若没有统一契约，会出现：

- 导出/分享把 harness 注入当成用户话
- resume 后 meta 丢失或重复
- Assembler 与 `project_for_llm` 双份折叠逻辑

需要一层薄而硬的 **Session SSOT ↔ Provider view** 边界，不实现具体栏/search/压缩算法。

## What Changes

- 规范性文档 +（若需）最小类型/标记：
  - **Session SSOT**：用户可见 transcript、Env、工具结果真值、冻结替换表等
  - **Provider view**：Assembler 输入；可含请求期投影的 harness-meta
  - **Harness-meta**：非终端用户话语（状态栏、search output 包装等）的标记与生命周期（持久 / 仅投影 / 可重建）
- `project_for_llm` / Assembler：**唯一** Env→LLM 折叠主路径（呼应 `src/AGENTS.md`）；禁止 infra 再折。
- 导出 / fork / resume：哪些 meta 带出、哪些重建——可测场景（至少文档场景 + 单测钩子）。
- 被引用方：`c1895`、`c1900`、`c1910` 实现时 MUST 遵守本契约（已加 `depends_on`）。

## Capabilities（意向）

- `agent-session` / `protocol` 消息边界
- 产品：会话与持久化（归档时一句）

## Impact

- 并行实现时有共同「存哪」语言。
- 减少状态栏写进 JSONL 却无法区分的事故。

## Out of scope

- 状态栏 UI/读数实现（→ `c1895`）
- tool_search（→ `c1900`）
- 压缩算法（→ `c1910`）
- 新插件式 meta 市场

## Parallel / depends

- **硬依赖**：`c1890`
- **下游**：`c1895`、`c1910`（及建议 `c1900` 遵守；`c1900` 以 Epoch 为主依赖，view 契约在正文 MUST 引用）
- 可与 `c1920`/`c1925`/`c1935` 并行

## Open Questions

- harness-meta 用独立 `AgentMessage` 变体还是 part/属性标记—— propose 时钉（影响持久化格式，谨慎）。
  - **上游意向（`c1895` Q7，2026-08-05）**：状态栏倾向独立 entry kind **`AgentStatusBar`** + 投影标签 **`<agent_status_bar>`**；本合约须消化该形状（或显式反驳）。
- Print 面导出默认是否剥离 meta—— 产品确认。

## Ethics

- risk_level: medium（持久化格式）
- prohibited_actions: 无标记地把框架注入持久化为普通 user；infra 平行折叠 Env
- required_evidence: 导出/resume 场景可测；折叠单路径
- refusal_contract: 不为此预挖插件 meta API
- escalation_policy: 改 JSONL 形状须显式迁移/版本策略
