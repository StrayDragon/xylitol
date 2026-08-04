---
depends_on:
  - c1890-add-responses-context-policy-assembler
---

# Assembler 布局决策可观测

> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md)（术语对照 §7）
> **书指针**：《深入理解 AI Agent》Ch2 上下文结构可观测性（姊妹仓 `ai-agent-book/book/chapter2.md`）；书语仅经 research §7 术语表映射，**禁止**写入 live specs。
> **自包含**：只加「本轮用了哪套布局规则」的观测；不新建检视台 UI。
> **工程约定（本波次）**：策略默认 **code-first**：`defaults.rs` 纯常量（改文件调试）；**不**新增 YAML 旋钮；**不**用 env 当未暴露配置面。用户面 YAML 仅既有字段（如 `api`）。真源见 [`c1880`](../archive/2026-08-04-c1880-update-responses-first-api-boundary/proposal.md)。

## Why

WirePolicy/compat、ContextPolicy 档、context epoch、tools 模式、date 放置、是否 chained 若只在内存，排障与多 agent 对照无法复现「发出去的规则集」。cache usage（`c1885`）回答「命中了吗」；本 change 回答「**按什么规则组装的**」。

## What Changes

- 每次 Responses 请求在既有观测栈记录结构化属性（意向）：`api`、`compat`、`context_epoch`（若已有）、`tools_mode`、`status_bar_mode`、`date_placement`（若有）、`replay_mode`（thinking）、`wire_mode`=`full_replay|chained`（后者待 `c1915`）。
- 落点：provider-trace 与/或 fastrace span 属性；遵守「观测失败不挡主路径」「默认不乱出站敏感 body」。
- 文档：`just obs-*` / skill 窄读指针加一例「如何对照 layout 决策与 cache 读数」。
- **禁止**自研 Inspect 检视台；**禁止**第二套 tracing 栈。

## Capabilities（意向）

- `infra-otel` / provider-trace
- `package-ai-bridge`（若在组装点打点）

## Impact

- 后续策略变更可对照「规则 ↔ usage」。
- 并行实现时减少「我以为开了 search 档」类失误。

## Out of scope

- Langfuse 数据集主闸
- UI 展示 layout 档（后置）
- 实现 Epoch/thinking/链式本身（只留属性槽；缺字段时省略）

## Parallel / depends

- **硬依赖**：`c1890`（Assembler 存在才有稳定打点点）
- 软依赖：`c1885`（对照 cache，不写入 depends_on）
- 属性名与 `c1920`/`c1925`/`c1915` 对齐；可先打点已知字段，后补

## Open Questions

- 是否在 debug 下附带 body 指纹（hash）而非全文—— 隐私默认。

## Ethics

- risk_level: low
- prohibited_actions: 默认全量出站 request body；tracing 双栈；自研检视台
- required_evidence: 单测或集成断言关键属性存在；坏观测配置不挡主路径
- refusal_contract: 不做「凡请求都进 Langfuse 全文」
- escalation_policy: 无
