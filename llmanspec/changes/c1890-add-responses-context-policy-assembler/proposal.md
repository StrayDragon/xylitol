---
depends_on:
  - c1880-update-responses-first-api-boundary
---

# ContextPolicy + ResponsesAssembler（可配置请求布局）

> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md)
> **自包含**：本草案写清缝与验收；实现时只认 Responses 主 wire。状态栏/tool_search/压缩为后续挂载点，本 change **不实现**它们的完整行为。
> **工程约定（本波次）**：策略默认 **code-first**：`defaults.rs` 纯常量（改文件调试）；**不**新增 YAML 旋钮；**不**用 env 当未暴露配置面。用户面 YAML 仅既有字段（如 `api`）。真源见 [`c1880`](../c1880-update-responses-first-api-boundary/proposal.md)。

## Why

今天 system / tools / messages 组装散落在 prompt 构建、ReAct、adapter 多处，策略无法按档切换，也无法对「同一 session 快照 → body」做字节级回归。需要一层：

```text
Session SSOT → ContextPolicy → ResponsesAssembler(flavor) → /v1/responses body
```

应用层仍可用 `AiBridgeMessage`；**wire 布局与缓存相关决策只在 Assembler**。Completions 走自己的薄路径，不共享 Responses 布局假设。Assembler **必须消费 `c1880` 的 flavor + capabilities**：同为 Responses 形似端，DeepSeek / llama.cpp / 官方等细节不同，用户可覆盖策略，禁止写死官方语义。

## What Changes

- 引入 `ContextPolicy`（名称以实现为准）：至少覆盖
  - 稳定段 vs 可变段的**切点配置**（默认可保持现状：cwd 可在 system；date 见下「日界占位」）
  - tools 暴露模式钩子：`full` | `search`（后由 `c1900` 实现 search）| 其它显式档
  - 状态栏模式钩子：`off` | `replace` | `append`（后由 `c1895` 实现）
  - 是否允许中途改 provider `tools` 表（默认策略写清；search 模式下应为否）
  - **日界（calendar-day）策略占位**（本 change 只留钩子与文档，不定最终算法；深挖在 `c1905`/`c1895`）：
    - 识别「同一 session 跨自然日 resume」时 date 放哪（system 刷新 vs 状态栏 vs 跨日 meta）；
    - Policy 预留枚举/配置键（如 `date_placement` / `day_boundary`），默认可先等价现状，但**禁止**后继 change 无处挂载。
  - **flavor 输入**：Policy/Assembler 读取 session 当前 model 的 `api` + `flavor` + capabilities（来自 `c1880`），作为布局与字段开关的输入之一。
- 引入 `ResponsesAssembler`：唯一构造 Responses JSON body（model/input/tools/instructions/store/…）。
  - **适配层**：按 flavor 决定字段子集、省略不支持的键、usage/扩展点交给后续 change；`generic` 走保守子集。
  - 用户 flavor/capabilities **覆盖**优先于内置预设。
- Golden / 单测：固定 session 投影 → body 稳定；切换 policy 档有可预期 diff；**至少两个 flavor**（如 official vs generic）同投影不同字段集可测。
- ReAct / infra 改为经 Assembler 发 Responses；禁止 adapter 内二次重排「业务布局」（flavor 差异在 Assembler/适配层消化，不回流改 `AgentMessage`）。

## Capabilities（意向）

- `agent-*`（投影与 policy 消费）
- `package-ai-bridge`（Responses 组装 + flavor 适配）
- `infra-config`（policy / flavor 配置）

## Impact

- `c1895`/`c1900`/`c1905`/`c1910`/`c1915` 可并行挂在本缝上（见各自 `depends_on`），并继承 flavor 开关。
- 实现时：本 change 交付「缝 + 默认行为等价现状 + 日界/flavor 占位」，后继 change 填策略。

## Out of scope

- 状态栏完整实现（→ `c1895`）
- tool_search / MCP 内部目录（→ `c1900`）
- 压缩冻结串（→ `c1910`）
- `previous_response_id`（→ `c1915`）
- 日界最终选型与实现（→ `c1905`/`c1895`；本 change 仅占位）
- 重写 Completions 布局优化
- 自动探测 flavor

## Parallel / depends

- **硬依赖**：`c1880`（Responses 默认边界、flavor、capabilities 配置位）
- 软建议：`c1885` 先或同迭代便于对照 cache，**不**写入 `depends_on`（避免阻塞并行）

## Open Questions

- Policy 存活位置：会话覆盖 vs 仅全局配置——与「运行时即时设置」关系在 propose 时钉。
- `instructions` 顶栏 vs `input` 内 system/developer：二选一 SSOT，避免双份。
- 日界占位键的默认值是否完全等同今日 `Current date:` 行为—— propose 时钉。

## Ethics

- risk_level: medium
- prohibited_actions: 为 Anthropic 设计 Assembler 主路径；无测试的布局漂移；**把所有 Responses 兼容端当成 OpenAI 官方字段全集**
- required_evidence: golden body 测；默认档行为与现状可对照；flavor 覆盖可测；日界配置键存在且有文档指针
- refusal_contract: 不把「命中率」写进 MUST；不承诺兼容端行为一致
- escalation_policy: 若默认档改变用户可见行为，须产品确认
