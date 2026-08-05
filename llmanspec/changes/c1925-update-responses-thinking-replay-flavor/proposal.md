---
depends_on:
  - c1880-update-responses-first-api-boundary
  - c1890-add-responses-context-policy-assembler
---

# Responses thinking / reasoning 回放 × WirePolicy/compat

> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md) §5.1（术语对照 §7）
> **书指针**：《深入理解 AI Agent》Ch2 思维链/工具环回放直觉（姊妹仓 `ai-agent-book/book/chapter2.md`）；书语仅经 research §7 术语表映射，**禁止**写入 live specs。
> **自包含**：多轮正确性（回放），不是可选优化；按 WirePolicy/compat 降级，禁止假设官方语义。
> **工程约定（本波次）**：策略默认 **code-first**：`defaults.rs` 纯常量（改文件调试）；**不**新增 YAML 旋钮；**不**用 env 当未暴露配置面。用户面 YAML 仅既有字段（如 `api`）。真源见 [`c1880`](../archive/2026-08-04-c1880-update-responses-first-api-boundary/proposal.md)。

## Why

Coding agent 多轮依赖 assistant 侧 thinking / reasoning 项回放（签名、encrypted_content、顺序）。今日有 pi 对齐路径，但不同 Responses 形似端（官方 / DeepSeek / llama.cpp / 网关）对：

- 是否返回可回放的 reasoning 项
- 缺字段 SSE
- 是否要求后续请求原样带回

差异很大。回放错误会破坏轨迹与工具环，严重性高于 cache 未命中。须在 Assembler 层按 **WirePolicy（compat + extra_policy）** 定义保留 / 剥离 / 降级合约。

## What Changes

- 规范性：**回放模式**枚举（意向）`preserve` | `strip` | `best_effort`，由 WirePolicy/compat 预设、用户可覆盖。
- Assembler 组装 input 时：thinking/reasoning 项顺序与现网 pi 对齐为默认（official）；`generic`/不支持端走 strip 或 best_effort，并诚实观测。
- ReAct 落盘：需要回放的签名/材料按模式写入 `AiBridgeMessage`；不支持时不假装写入。
- 单测：至少 official preserve 与 generic strip（或 best_effort）两条 golden。
- 与 `c1915` 关系：链式续跑仍依赖正确回放/全量重放；本 change **不**实现链式。

## Capabilities（意向）

- `package-ai-bridge`（thinking / Responses 项）
- `agent-runtime`（落盘）
- WirePolicy / extra_policy（已归档 `c1880`）

## Impact

- 多轮工具调用在兼容端上行为可预期、可关回放。
- 后续 Epoch/链式以「回放合约已定」为前提更安全。

## Out of scope

- previous_response_id（→ `c1915`）
- 状态栏 / MCP search
- Anthropic thinking blocks 真实现
- 训练数据收集模式下的「拒绝修补」策略（可后置）

## Parallel / depends

- **硬依赖**：`c1880`、`c1890`
- 可与 `c1920`/`c1930`/`c1935` 并行；注意 bridge thinking 文件所有权

## Open Questions

- `best_effort` 失败时是否降级为 strip 并记诊断—— propose 时钉。
- 与 `store`/encrypted_content 隐私—— 与 `c1915` Open Questions 交叉引用。

## Ethics

- risk_level: medium
- prohibited_actions: 对不支持端伪造 encrypted 回放；把官方 include 列表强加给所有 compat/WirePolicy
- required_evidence: 双 WirePolicy golden；缺能力时不崩主路径
- refusal_contract: 不承诺一切兼容端可完整回放 thinking
- escalation_policy: 默认从 preserve 改为 strip 须确认（影响强模型行为）
