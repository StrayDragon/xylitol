---
depends_on:
  - c1890-add-responses-context-policy-assembler
---

# Context Epoch（前缀 / 工具世代冻结）

> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md)（术语对照 §7）
> **书指针**：《深入理解 AI Agent》Ch2「KV Cache 友好的上下文设计」前缀世代直觉（姊妹仓 `ai-agent-book/book/chapter2.md`）；书语仅经 research §7 术语表映射，**禁止**写入 live specs。
> **自包含**：统一「谁在何时允许改稳定前缀」；供 `c1900`/delayed `c1915`/换模断链共用，避免各 change 私自定义「工具世代」。
> **工程约定（本波次）**：策略默认 **code-first**：`defaults.rs` 纯常量（改文件调试）；**不**新增 YAML 旋钮；**不**用 env 当未暴露配置面。用户面 YAML 仅既有字段（如 `api`）。真源见 [`c1880`](../archive/2026-08-04-c1880-update-responses-first-api-boundary/proposal.md)。

## Why

仅禁止 MCP 热改 `tools` 不够。下列操作都会改「模型以为稳定」的前缀或能力集：

- `/reload`（skills、AGENTS.md、prompt context）
- MCP settle / `set_tools`
- `runtime_policy_fragments` 变化
- NextTurn 换模（WirePolicy/compat / thinking 档变化）

没有显式 **context epoch**（可拆 prompt_epoch / tools_epoch），Assembler、tool_search、链式续跑无法对「前缀是否仍可比」「是否必须 full replay」达成一致。

## What Changes

- Session（或等价运行时状态）维护 **context epoch**（最小：单一单调计数；或拆 `prompt_epoch` + `tools_epoch`，propose 时钉）。
- 定义 **bump 表**（规范性）：哪些 API/用户动作 bump 哪一类 epoch；哪些禁止静默 bump（须用户可见 cue / 下轮预告）。
- ContextPolicy / Assembler **读取当前 epoch** 并写入本轮布局决策（供 delayed `c1935` 观测）。
- NextTurn 换模：默认 bump（或按「WirePolicy / extra_policy 是否实质变化」）；规则写清，与产品「下轮生效」一致。
- 文档：epoch 与 Prompt Cache「前缀变了」的关系（工程语义，不追命中率）。

## Capabilities（意向）

- `agent-session` / `agent-runtime`
- ContextPolicy 键
- 产品：运行时即时设置 / 压缩与上下文（归档时一句）

## Impact

- `c1900` 的「工具世代」、`c1915` 的断链条件 **MUST** 引用本契约，不再平行发明。
- `/reload` 与 MCP settle 行为可测、可解释。

## Out of scope

- tool_search 实现（→ `c1900`）
- previous_response_id（→ delayed `c1915`）
- 状态栏内容（→ delayed `c1895`）
- 自动探测网关

## Parallel / depends

- **硬依赖**：`c1890`
- **下游应依赖本 change**：`c1900`；delayed `c1915` 日后接；换模断链语义以本为准
- 可与 `c1925`/`c1930` 并行；`c1935` delayed

## Open Questions

- 单 epoch vs 双 epoch（prompt/tools）—— propose 时钉（双更准、单更简单）。
- bump 是否对用户展示（chrome 短 cue）—— 与 TUI 词汇对齐。

## Ethics

- risk_level: medium
- prohibited_actions: 静默 bump 导致用户以为前缀未变；各下游 change 再定义冲突的「世代」
- required_evidence: bump 表 + 单测；reload/换模路径有明确 epoch 变化
- refusal_contract: 不把 epoch 当成自动抬 cache 的手段
- escalation_policy: 若 bump 改变开箱默认可见行为，须产品确认
