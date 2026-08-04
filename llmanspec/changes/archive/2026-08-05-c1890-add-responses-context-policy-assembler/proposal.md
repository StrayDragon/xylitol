---
depends_on:
- c1880-update-responses-first-api-boundary
branch: sdd/c1890-add-responses-context-policy-assembler
base_sha: 9008e7b94c4fdde14db21de4f2af9378d15781d3
checkpointed: true
checkpoint_sha: 1c8c4d8ba3be207ca02ff3526d00cf9fb82de360
---

# ContextPolicy + ResponsesAssembler（可配置请求布局）

> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md) §1–2、§5、§7（术语对照）
> **书指针**：《深入理解 AI Agent》Ch2（姊妹仓 `ai-agent-book/book/chapter2.md`）—「上下文：决定 Agent 能力上限」「Agent 如何调用大模型」「KV Cache 友好的上下文设计」；书语仅经 research §7 术语表进入工程名，**禁止**写入 live specs。
> **自包含**：本草案写清缝与验收；实现时只认 Responses 主 wire。状态栏/tool_search/压缩为后续挂载点，本 change **不实现**它们的完整行为。
> **工程约定（本波次）**：策略默认 **code-first**：`defaults.rs` 纯常量（改文件调试）；**不**新增 YAML 旋钮；**不**用 env 当未暴露配置面。用户面 YAML 仅既有字段（如 `api`）。WirePolicy 真源见已归档 [`c1880`](../archive/2026-08-04-c1880-update-responses-first-api-boundary/proposal.md)。

## Why

今天 system / tools / messages 组装散落在 prompt 构建、ReAct、adapter 多处，策略无法按档切换，也无法对「同一 session 快照 → body」做字节级回归。需要一层：

```text
Session SSOT → ContextPolicy → ResponsesAssembler(WirePolicy) → /v1/responses body
```

应用层仍可用 `AiBridgeMessage`；**wire 布局与缓存相关决策只在 Assembler**。Completions 走自己的薄路径，不共享 Responses 布局假设。Assembler **必须消费 `c1880` 的 `WirePolicy`（`compat` + `extra_policy`）与既有 `api`**：同为 Responses 形似端，DeepSeek / llama.cpp / 官方等细节不同；本波默认板 code-first，禁止写死官方字段全集。

## What Changes

- 引入 `ContextPolicy`（名称以实现为准）：至少覆盖
  - 稳定段 vs 可变段的**切点配置**（默认可保持现状：cwd 可在 system；date 见下「日界占位」）
  - tools 暴露模式钩子：`full` | `search`（后由 `c1900` 实现 search）| 其它显式档
  - 状态栏模式钩子：`off` | `replace` | `append`（后由 `c1895` 实现）
  - 是否允许中途改 provider `tools` 表（默认策略写清；search 模式下应为否）
  - **日界（calendar-day）策略占位**（本 change 只留钩子与文档，不定最终算法；深挖在 `c1905`/`c1895`）：
    - 识别「同一 session 跨自然日 resume」时 date 放哪（system 刷新 vs 状态栏 vs 跨日 meta）；
    - Policy 预留枚举/配置键（如 `date_placement` / `day_boundary`），默认可先等价现状，但**禁止**后继 change 无处挂载。
  - **WirePolicy 输入**：Policy/Assembler 读取当前请求的 `api` + `WirePolicy`（`compat` / `extra_policy`，来自 `c1880` defaults 或测试覆盖），作为布局与字段开关的输入之一。
- 引入 `ResponsesAssembler`：唯一构造 Responses JSON body（model/input/tools/instructions?/store/…）。
  - **适配层**：按 `WirePolicy.compat`（及 `extra_policy` 位）决定字段子集、省略不支持的键、usage/扩展点交给后续 change；`Compat::Generic` 走保守子集。
  - 测试可构造 `WirePolicy { .. }` **覆盖**优先于内置默认板（与 `c1880` 一致；本波无 YAML 覆盖面）。
- Golden / 单测：固定 session 投影 → body 稳定；切换 policy 档有可预期 diff；**至少两套 WirePolicy**（默认 generic 保守子集 vs 显式打开某 `extra_policy` 位）同投影不同字段集可测。
- ReAct / infra 改为经 Assembler 发 Responses；禁止 adapter 内二次重排「业务布局」（compat/extra_policy 差异在 Assembler/适配层消化，不回流改 `AgentMessage`）。

## Capabilities（意向）

- `agent-*`（投影与 policy 消费）
- `package-ai-bridge`（Responses 组装 + WirePolicy 适配）
- （可选）`agent-runtime` / prompt 缝：调用 Assembler，不扩 YAML schema

## Impact

- `c1895`/`c1900`/`c1905`/`c1910`/`c1915`/`c1920`/`c1925`/`c1930`/`c1935` 可并行挂在本缝上（见各自 `depends_on`），并继承 WirePolicy 开关。
- 实现时：本 change 交付「缝 + 默认行为等价现状 + 日界/WirePolicy 占位」，后继 change 填策略。

## Out of scope

- 状态栏完整实现（→ `c1895`）
- tool_search / MCP 内部目录（→ `c1900`）
- 压缩冻结串（→ `c1910`）
- `previous_response_id`（→ `c1915`）
- 日界最终选型与实现（→ `c1905`/`c1895`；本 change 仅占位）
- 重写 Completions 布局优化
- 自动探测网关能力 / 新 YAML 旋钮

## Parallel / depends

- **硬依赖**：已归档 `c1880`（Responses 默认边界、`WirePolicy` / `compat` / `extra_policy`）
- 软建议：已归档 `c1885` 便于对照 cache，**不**写入 `depends_on`（避免阻塞并行）

## Open Questions

### Resolved（propose 钉）

- **Q1 Policy 存活位置**：选 **A — 仅全局 code-first 默认板**（`ContextPolicy::default()` + `defaults.rs`）。本 change **不**做会话覆盖、**不**做 YAML。会话级即时设置若需要，后置 change 再开闭。
- **Q2 `instructions` vs `input` 内 system/developer**：选 **A — 保持现状 SSOT**：经 `AiBridgeGenerateOptions.system_prompt` → Assembler/`prepend_system_prompt_item`（thinking on → `developer`，off → `system`）；**不**并行写顶栏 `instructions` 第二份。若日后改信道，须单测 + 产品确认。
- **Q3 日界占位默认**：选 **A — 完全等同今日** `build_system_prompt` 的 date 行为（占位键存在但默认值 = 现状）；最终算法留给 `c1905`/`c1895`。

### Open

- （无）上列已收束；细节落 `design.md` / `tasks.md`。

## Ethics

- risk_level: medium
- prohibited_actions: 为 Anthropic 设计 Assembler 主路径；无测试的布局漂移；**把所有 Responses 兼容端当成 OpenAI 官方字段全集**；把书语写进 live specs
- required_evidence: golden body 测；默认档行为与现状可对照；WirePolicy 覆盖可测；日界配置键存在且有文档指针
- refusal_contract: 不把「命中率」写进 MUST；不承诺兼容端行为一致
- escalation_policy: 若默认档改变用户可见行为，须产品确认
