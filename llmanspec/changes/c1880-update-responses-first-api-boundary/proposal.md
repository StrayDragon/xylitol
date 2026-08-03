---
depends_on: []
---

# Responses 默认主路径 + Completions 显式类型 + Anthropic 桩

> **调研底稿**（非本 change）：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md)（含 §5.1 flavor）
> **波次**：Wave A（可与 `c1885` 并行）
> **自包含**：本草案单独可认领；不依赖其它未归档 change。

## Why

要把上下文布局、Prompt Cache、工具披露等优化做到可配置、可复现，必须先钉死：**主交付只围绕 OpenAI Responses（含兼容端）**。若继续为 Anthropic Messages / Completions 共用同一套「断点与布局」假设，Assembler 会被双方言拖成不可维护的中间层。

同时完全删掉 Completions 有耦合风险：未来若某网关或本地栈只稳 Completions，系统会过度绑死 Responses 形状。折中：**Completions 保留为显式 `api` 类型**（配置一眼可见），Anthropic **留桩**不进默认。

另一易忘坑：**多个 LLM provider 都「实现了 OpenAI Responses 风格」≠ 行为一致**（例：DeepSeek / llama.cpp / 网关 vs 官方 OpenAI）。cache 字段、tool_search、SSE 缺字段、`previous_response_id` 等细节可迥异。必须保留 **flavor（实现口味）配置 + 适配层**，允许用户覆盖默认策略；禁止写死「凡 Responses 即官方语义」。

## What Changes

- 默认装配 / 文档 / 开箱路径：`api` 缺省 = Responses（或等价 `openai-responses`）。
- 模型/provider 配置：**显式**允许 `openai-responses` 与 `openai-completions` 两种 OpenAI 族类型；Completions 不享受 Responses 专用优化，但可工作（遗留/逃生）。
- Anthropic Messages：保留模块骨架 + 清晰注释/桩（「未来 Claude 等」）；默认 CI/开箱不装配；**禁止**为 Anthropic `cache_control` 反向设计 Responses Assembler。
- 引入配置维 **`flavor`**（名称以实现为准；例：`openai-official` / `deepseek` / `llamacpp` / `generic`）：
  - 与 `api` 正交：`api` = 协议族，`flavor` = 同族下的实现口味；
  - 首版提供少量内置预设 + **用户可覆盖**（capabilities / 字段策略）；
  - **不做**自动探测；观测可记下本次实际 flavor，便于排障。
- 模型档案 **capabilities 配置块**（手写；可由 flavor 预设再覆盖），字段意向例如：`prompt_cache_usage`、`tool_search`、`defer_loading`、`previous_response_id`、`prompt_cache_key`。
- 适配层契约（本 change 钉边界，Assembler 细装在 `c1890`）：同一应用投影进入 bridge 后，**按 flavor 选择**发往上游的字段子集、usage 映射、降级（缺能力则关优化，不假装支持）。
- 同步产品文：`docs/architecture/多厂商模型.md` / 相关 AGENTS：主优化轴 = Responses；Completions = 显式类型；Anthropic = 后置桩；**兼容端须配 flavor，勿假设官方语义**。

## Capabilities（意向）

- `package-ai-bridge`（装配/类型边界/flavor 适配缝）
- `infra-config` / `runtime-config`（api、flavor、capabilities）
- 产品叙事：`多厂商模型` architecture（归档时迁）

## Impact

- 自定义模型（含 llama.cpp / DeepSeek 等 Responses 形似端）可显式选 flavor，避免被官方策略误伤。
- Completions 用户须显式配置，避免误以为享受 Responses 缓存/tool_search 布局。
- 后续 Assembler / cache / tool_search / 链式续跑 **必须读 flavor+capabilities**，不得只认 `api == responses`。

## Out of scope

- 实现 ContextPolicy / Assembler 全量（→ `c1890`；本 change 只钉 flavor/capabilities 配置与适配层边界）
- cache usage 映射细节（→ `c1885`，但须按 flavor 决定是否期望有 cached_tokens）
- 删除 Completions 代码史或 Anthropic 文件物理删除
- 自动探测网关是否支持某 capability / flavor
- 图片/视频多模态传输选型
- 为每个网关手写完整第二套 HTTP 栈（仍走官方 SDK + byot/宽松 SSE 等既有开闭）

## Parallel / depends

- `depends_on: []`
- 可与 `c1885` 同波；`c1890`/`c1900`/`c1915` 依赖本 change 归档（或行为已落地）

## Open Questions

- capabilities / flavor 挂在 `models.*` 还是 `providers.*`（或两者合并视图）—— propose 时钉。
- Completions 是否仍跑最小回归，还是仅编译 + 手工冒烟—— propose 时钉。
- 内置 flavor 最小集合与 `generic` 默认降级表—— propose/design 钉。

## Ethics

- risk_level: medium（收窄默认交付）
- prohibited_actions: 为 Anthropic 断点语义改 Responses 主布局；静默把 Completions 当 Responses 优化；**假设所有 Responses 兼容端 ≡ OpenAI 官方**
- required_evidence: 默认路径只装配 Responses；显式 Completions 仍可选；Anthropic 默认不进开箱；配置可设 flavor 并覆盖 capabilities
- refusal_contract: 不承诺「所有厂商同一套 cache 断点 API」或「凡 Responses 行为一致」
- escalation_policy: 若要物理删除 Anthropic/Completions 源文件，须单独确认
