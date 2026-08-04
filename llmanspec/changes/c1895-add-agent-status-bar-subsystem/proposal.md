---
depends_on:
  - c1890-add-responses-context-policy-assembler
  - c1930-update-session-provider-view-contract
---

# Agent 状态栏子系统（可插拔 / 可验证）

> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md) §3；书 Ch2 状态栏两实现。
> **自包含**：只交付状态栏机制；不实现 MCP search / 压缩。持久化/投影标记遵守 `c1930`。
> **工程约定（本波次）**：策略默认 **code-first**：`defaults.rs` 纯常量（改文件调试）；**不**新增 YAML 旋钮；**不**用 env 当未暴露配置面。用户面 YAML 仅既有字段（如 `api`）。真源见 [`c1880`](../c1880-update-responses-first-api-boundary/proposal.md)。

## Why

状态栏不是银弹：coding agent 下 cwd/date 进 system 常常正确；但工具计数、TODO、git 等**高变读数**若塞进 system 或每轮无策略追加，会在「cache」与「注意力/冗余」之间失控。需要**独立、默认可关、可测**的子系统，由 ContextPolicy 选择 `off` | `replace` | `append`。

## What Changes

- StatusBar **provider 接口**：输入 = 代码可观测状态（会话计数、可选 git/TODO 等）；输出 = 结构化键值（禁止散文堆砌为默认）。
- 三种模式（配置）：
  - `off`（推荐默认）
  - `replace`：每轮仅保留最新一条 meta（接受末尾局部 cache 失效）
  - `append`：只追加不删（cache 友好；须文档警告陈旧条与注意力）
- 注入经 Assembler / Policy，**不**散落改 `build_system_prompt` 特例逻辑（system 内稳定 env 仍可由 `c1905` 管）。
- 验证：给定假状态 → 栏内容单测；模式切换可消融；可选「读数 + 短策略片段」成对配置。
- **禁止**用 LLM 批量扫历史生成栏。

## Capabilities（意向）

- `agent-*`（status bar）
- 配置 / ContextPolicy 键
- 产品文：压缩与上下文 / 新 architecture 短节（归档时）

## Impact

- 弱模型/长轨迹可按需打开；默认不强迫。
- 与 cache 优化解耦：开栏不等于追求命中。

## Out of scope

- 把 cwd/date **强制**迁出 system（本仓场景默认可留；`c1905` 可标 stable）
- tool_search（→ `c1900`）
- 子 agent 字节级对齐父栏（后置）

## Parallel / depends

- **硬依赖**：`c1890`、`c1930`
- 可与同层无硬依赖冲突的 change 并行（不同文件/模块优先）

## Open Questions

- meta 是否写入持久 transcript，还是仅请求时投影—— propose 时钉（影响导出/resume）。
- 首版提供哪些内置读数最小集。
- **日历日 `date` 是否作为状态栏读数（与 `c1905` 联调深挖）**：隔日 resume 同一 session 时，system 内 date 过时 vs 改写前缀失效，是已知坑。若选型为「date 走栏」，须定 replace vs append（append 会堆多日陈旧 date，模型须认最新条）以及是否写入 transcript。若选型仍留 system，本 change 可不承载 date，但 design 须写明「不负责日界」以免两 change 都不管。指针：research §1；`c1905` Open Questions。

## Ethics

- risk_level: medium（高信任注入面）
- prohibited_actions: LLM 维护栏；不可关的隐式每轮 append；把外部不可信全文写入栏
- required_evidence: off/replace/append 可测；默认 off 或等价不强迫
- refusal_contract: 不宣称状态栏普遍提升正确率
- escalation_policy: 若默认改为 on，须用户确认
