---
depends_on: []
branch: sdd/c1885-add-responses-cache-usage-honesty
base_sha: 777caa15d18bd30e8a21931f041e828093e11120
checkpointed: true
checkpoint_sha: 98c02592743f799ff45b2690a154328312591732
---

# Responses usage：Prompt Cache 诚实透出

> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md)
> **自包含**：只改 usage 映射与产品诚实叙事；不依赖 Assembler / Responses-only 边界落地。
> **工程约定（本波次）**：策略默认 **code-first**：`defaults.rs` 纯常量（改文件调试）；**不**新增 YAML 旋钮；**不**用 env 当未暴露配置面。用户面 YAML 仅既有字段（如 `api`）。WirePolicy 真源见已归档 [`c1880`](../archive/2026-08-04-c1880-update-responses-first-api-boundary/proposal.md)。

## Why

当前 Completions 路径已能映射 `cached_tokens`，Responses 曾长期把 `cache_read` 置 0，后续「稳定前缀 / tool_search / 状态栏」策略没有尺子可对照。`c1880` 已落地 `WirePolicy` 闸门与条件映射骨架；**本 change 把尺子做成可区分的诚实语义**（三态 + 默认可观测），目标是**可观测与诚实**，不是最大化命中率。

同为 Responses 形似端，**未必回报** cache 细节。映射须按 `WirePolicy.expects_prompt_cache_usage`：**期望且有则透出，期望且无则标未回报，不期望则标不适用**——禁止对 DeepSeek/llama.cpp 等伪造官方字段语义，也禁止把「没字段」当成「没命中」。

## What Changes

- Responses usage：引入三态 `PromptCacheRead`（名以实现为准）；映射 `input_tokens_details.cached_tokens`（及官方存在的 write 类字段，若无则保持 0/NotApplicable 并文档说明）。
- **`PROMPT_CACHE_USAGE` 默认 true**（改 `defaults.rs`；修订 live `pab19` **仅翻**该位）：有字段 → `Tokens`，无 → `NotReported`；显式关闸 → `NotApplicable`。
- **观测**：trace / Langfuse（及既有 OTel 路径）透出三态；`usage_details.cache_read` 仅在 `Tokens(n)` 时写数字。
- **不做 TUI**（延后草案 [`c1940`](../../../delayed-changes/tui/c1940-add-tui-prompt-cache-footer/proposal.md)）。
- **不做** `prompt_cache_key` 请求透传（后置）。
- 单测：fixtures → 三态；兼容端无细节 → `NotReported`（默认开闸下）。

## Capabilities（意向）

- `package-ai-bridge`（usage / DTO）
- 观测：trace / Langfuse 属性对齐（仅当透出需合约）
- 产品文：roadmap「上下文缓存」M1 叙事（诚实尺子）；**不**动 TUI chrome

## Impact

- 后续 ContextPolicy / tool_search 可在观测侧对照真实 cache 读数。
- 排障经 Langfuse/trace 可见「吃到缓存 / 未回报 / 不适用」。
- TUI 本 change **无**新 chrome（见 `c1940`）。

## Out of scope

- TUI footer / chrome（→ [`c1940`](../../../delayed-changes/tui/c1940-add-tui-prompt-cache-footer/proposal.md)）
- `prompt_cache_key` 请求透传与键策略（明确后置）
- 改变 system/tools 布局或状态栏子系统（→ `c1890`/`c1895`）
- 强制开启某 cache **策略**（布局侧）
- Anthropic `cache_creation_*` 主路径（桩即可）
- Eval 回归闸（后置）

## Parallel / depends

- `depends_on: []`（硬依赖无；语义承接已归档 `c1880` WirePolicy）
- 延后 TUI：[`c1940-add-tui-prompt-cache-footer`](../../../delayed-changes/tui/c1940-add-tui-prompt-cache-footer/proposal.md)
- Specs：修订 `package-ai-bridge` pab19（仅 `prompt_cache_usage` 默认 true）

## Open Questions

### Resolved

- **Q1 最小诚实交付**：选 **A — 三态 provenance**。须区分「命中 N」「回报 0」「未回报/不适用」；禁止把「没字段」当成「没命中」。映射仍受 `WirePolicy.expects_prompt_cache_usage` 闸门约束（真源 `c1880` 已归档）。`cache_read: u64` 单字段混义不够。
- **Q2 类型落点**：选 **A — 一等枚举**（意向名 `PromptCacheRead` / 以实现为准）：`NotApplicable | NotReported | Tokens(u64)`（含 `Tokens(0)`）。`cache_read: u64` 可保留为派生兼容读数（仅 `Tokens(n)→n`，其余→0）供 accounting/compact 求和；禁止只靠 `Option<u64>`（糊掉 NotApplicable vs NotReported）。
- **Q3 闸门默认**：选 **B — `PROMPT_CACHE_USAGE` 默认 true**。Responses 路径默认期望 cache 细节；有字段 → `Tokens(n)`，无 → `NotReported`。动机：本地/联调测试必须能看见诚实读数与「未回报」。须修订 live `pab19`（仅翻 `prompt_cache_usage` 默认）。`NotApplicable` 留给显式关闸。方言端 NotReported 噪音可接受。
- **Q4 产品面**：选 **A — bridge DTO/usage/单测 + trace/Langfuse 顺带三态；不做 TUI footer**。延后 TUI 另立 draft [`c1940`](../../../delayed-changes/tui/c1940-add-tui-prompt-cache-footer/proposal.md)。
- **Q5 `prompt_cache_key`**：选 **A — 整段后置**。本 change 不接请求透传、不定键策略；仍由 `allows_prompt_cache_key`（默认 false）闸着。范围 = **resp/usage 三态诚实 + 观测透出**。

### Open

- （无）深挖决策已收束；下一步 propose：`design.md` / `tasks.md` + Branch binding + Specs landing。

## Ethics

- risk_level: low
- prohibited_actions: 伪造 cache hit；把启发式标成 Api cache
- required_evidence: Responses fixture 映射单测；未回报时诚实标注
- refusal_contract: 不把 hit rate 写成产品 KPI
- escalation_policy: 无
