---
depends_on:
  - c1890-add-responses-context-policy-assembler
  - c1930-update-session-provider-view-contract
---

# Agent 状态栏子系统（可插拔 / 可验证）

> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md) §3（术语对照 §7）
> **书指针**：《深入理解 AI Agent》Ch2「Agent 状态栏：通过元信息增强 Agent 轨迹管理」（姊妹仓 `ai-agent-book/book/chapter2.md`）；书语仅经 research §7 术语表映射，**禁止**写入 live specs。
> **自包含**：本波交付 **Lane Runtime**（Harness 观测 + 注入）与 **Lane Agent 扩展壳**（接口/空实现）；完整 TODO/计划业务 → [`c1896`](../c1896-add-status-bar-agent-lane/proposal.md)。不实现 MCP search / 压缩。持久化/投影标记遵守 `c1930`。
> **工程约定（本波次）**：策略默认 **code-first**：`defaults.rs` 纯常量（改文件调试）；**不**新增 YAML 旋钮；**不**用 env 当未暴露配置面。用户面 YAML 仅既有字段（如 `api`）。真源见 [`c1880`](../archive/2026-08-04-c1880-update-responses-first-api-boundary/proposal.md)。

## Why

状态栏不是银弹：coding agent 下 cwd/date 进 system 常常正确；但工具计数、TODO、git 等**高变读数**若塞进 system 或每轮无策略追加，会在「cache」与「注意力/冗余」之间失控。需要**独立、可测**的子系统，由 ContextPolicy 选择 `off` | `replace` | `append`。

## What Changes

- **双列地基（深挖 Q1，已钉）**：
  - **Lane Runtime**：代码可观测状态 → 结构化键值；注入经 Assembler / Policy；内置 `refresh` 类工具 = **触发代码重算快照**（禁止 LLM 批量扫历史写栏）。
  - **Lane Agent**：可扩展 typed 尾插通道；本波只留扩展接口/空壳；业务 TODO → `c1896`。
- StatusBar **provider 接口**（Runtime）：输入 = 代码可观测状态；输出 = 结构化键值（禁止散文堆砌为默认）。
- Runtime 默认模式：**`append`（深挖 Q2 已钉）**——**不**扫描轨迹中已有 status message；每轮在末尾直接追加最新快照（「盲目尾插」）。`replace` / `off` 仍可切。
- **持久化（深挖 Q3 已钉）**：**全部写入 session transcript**（SSOT），不是仅请求时投影。理由：常用 provider 的 KV / Prompt Cache 依赖跨请求前缀字节稳定；仅投影等于每轮换掉末尾条，破坏「只追加」命中。导出 / resume / compact（`c1897`）均可见这些特殊标记消息。
- 注入经 Assembler / Policy，**不**散落改 `build_system_prompt` 特例逻辑（system 内稳定 env 仍可由 `c1905` 管）。
- 验证：给定假状态 → 栏内容单测；模式切换可消融；可选「读数 + 短策略片段」成对配置。
- **禁止**用 LLM 批量扫历史生成权威栏。

## Capabilities（意向）

- `agent-*`（status bar / runtime lane）
- ContextPolicy `status_bar_mode` 真消费
- 产品文：压缩与上下文 / 新 architecture 短节（归档时）

## Impact

- 弱模型/长轨迹可按需打开；默认策略以深挖钉板为准。
- 与 cache 优化解耦：开栏不等于追求命中。
- Agent 列扩展点避免后继挤进 Runtime。

## Out of scope

- 把 cwd/date **强制**迁出 system（本仓场景默认可留；`c1905` 可标 stable）
- tool_search（→ `c1900`）
- 完整 TODO / 即时计划产品形态（→ `c1896`；本波仅扩展壳）
- 压缩时对特殊标记 status message 的保留策略（→ `c1897`；本波只保证可识别标记）
- 子 agent 字节级对齐父栏（后置）

## Parallel / depends

- **硬依赖**：`c1890`（已归档）、`c1930`
- **分流草案**：
  - [`c1896`](../c1896-add-status-bar-agent-lane/proposal.md)（Agent 列；sourced_from 本 change）
  - [`c1897`](../c1897-update-compaction-status-bar-messages/proposal.md)（压缩 × status；sourced_from 本 change）
- 可与同层无硬依赖冲突的 change 并行（不同文件/模块优先）

## Open Questions

### 已解决

- **Q1 双列范围（2026-08-05）**：选 **双列 + 本波只通 Runtime**；Agent 列留可扩展接口/空壳；TODO 业务形态未定 → 想法写入 `c1896`（标记 sourced_from 本 change）。
- **Q2 Runtime 默认模式（2026-08-05）**：选 **`append` = 盲目尾插**（不查看轨迹中已有 status message，直接追加到末尾）。`replace`/`off` 仍为可切档。压缩时陈旧 status 堆积 → 策略延后调研，写入 `c1897`（最多保留一条 vs 全不保留，未定）。
- **Q3 持久化（2026-08-05）**：选 **全部持久进 transcript**（非仅请求投影）。动机：保住常用 LLM provider 的跨请求 KV / Prompt Cache（append 前缀稳定）；仅投影会每轮替换末尾条、破坏命中。`c1897` 因此更关键。Agent 列（`c1896`）默认同源持久，除非后继另钉。

### 待钉

- 首版 Runtime 内置读数最小集。
- **日历日 `date` 是否作为状态栏读数（与 `c1905` 联调深挖）**：隔日 resume 同一 session 时，system 内 date 过时 vs 改写前缀失效，是已知坑。若选型为「date 走栏」，须定 replace vs append 以及是否写入 transcript。若选型仍留 system，本 change 可不承载 date，但 design 须写明「不负责日界」。指针：research §1；`c1905` Open Questions。
- TUI / 导出是否向用户展示 status 条（vs 仅模型可见）——可后置。

## Ethics

- risk_level: medium（高信任注入面；且持久后进入导出/resume）
- prohibited_actions: LLM 维护权威栏；把外部不可信全文写入栏；不可审计的隐式投毒通道
- required_evidence: off/replace/append 可测；Runtime provider 可消融；append 路径不依赖「扫旧 status」；持久条目带稳定特殊标记
- refusal_contract: 不宣称状态栏普遍提升正确率
- escalation_policy: 若默认从 append 改为更强侵入策略，须用户确认
