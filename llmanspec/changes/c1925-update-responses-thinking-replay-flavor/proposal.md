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

- 规范性：**回放模式**枚举 `preserve` | `strip` | `best_effort`，落在 **`ExtraPolicy.reasoning_replay`**（code-first）；`Compat` 只做预设映射，**不**扩 YAML/env。
- Assembler 组装 input：按模式回放 / 剥离；顺序保持现网 pi（reasoning 项在 assistant/tool 前）。
- **本波默认行为保持现状 ≈ preserve**（有合法 signature 则回放）；先立旋钮与双 golden，**不**把 `WirePolicy::default()` 默默改成 strip（另波须确认，见 Ethics）。
- ReAct 落盘：需要回放的签名/材料按模式写入；不支持时不假装写入；**本波继续允许**含 `encrypted_content` 的 signature 整包进 JSONL（敏感文档化）；**不**改 `store:true`。
- 单测：至少 **preserve** 与 **strip**（或 best_effort）两条 WirePolicy golden。
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
- 可与 `c1930` 并行；`c1920`/`c1935` delayed；注意 bridge thinking 文件所有权

## Decisions（explore 2026-08-05 · 已钉）

1. **默认回放**：本波 **保持现状 ≈ preserve**；先加 `reasoning_replay` 旋钮 + 双 golden，**不**改 generic 默认行为。日后默认 preserve→strip 另开确认波。
2. **`best_effort` 失败**：降级 **strip** + 记诊断（trace/diagnostics）；主路径不崩；**禁止**伪造 encrypted。
3. **旋钮落点**：`ExtraPolicy.reasoning_replay: Preserve | Strip | BestEffort`；`Compat` 仅预设映射。
4. **encrypted / 本地 JSONL**：本波 **继续允许** signature 整包落盘（工具环正确性优先）；文档标敏感；**不**为本波改 `store:true`；与 delayed `c1915` 隐私/链式 Q 交叉引用、本 change 不解。

## Open Questions

- （已清空；上表为 explore 拍板。propose 时落入 design/specs。）

## Ethics

- risk_level: medium
- prohibited_actions: 对不支持端伪造 encrypted 回放；把官方 include 列表强加给所有 compat/WirePolicy；静默把默认从 preserve 改为 strip
- required_evidence: 双 WirePolicy golden；缺能力 / best_effort 失败时不崩主路径且有诊断
- refusal_contract: 不承诺一切兼容端可完整回放 thinking
- escalation_policy: 默认从 preserve 改为 strip 须确认（影响强模型行为）
