---
depends_on:
- c1890-add-responses-context-policy-assembler
branch: sdd/c1905-update-system-prompt-stable-volatile-split
base_sha: f1611239f68c0200259abe6f67724ed4fa17ba77
checkpointed: true
checkpoint_sha: f1611239f68c0200259abe6f67724ed4fa17ba77
---

# System prompt：稳定 / 可变切分（配置化，不教条）
> **一句话**：system 保持 stable；date/cwd 以 **状态栏族特殊类型 `session_env`** 注入；完整栏留给 c1895
> **当前排序**：#13（2026-08-10 自 delayed-changes 升格入 active）


> **已升格（2026-08-10）**：自 delayed-changes 移入 active 待处理队列，当前排序 **#13**。


> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md) §5–6（术语对照 §7）；一手深挖 [`research/stable-volatile-split-2026.md`](./research/stable-volatile-split-2026.md)
> **书指针**：《深入理解 AI Agent》Ch2「KV Cache 友好的上下文设计」「提示工程」（姊妹仓 `ai-agent-book/book/chapter2.md`）；书语仅经 research §7 术语表映射，**禁止**写入 live specs。
> **自包含**：prompt 片段分类 + **session_env 状态栏 bootstrap**；不实现完整状态栏 Lane / MCP search。
> **下游栏**：[`c1895`](../c1895-add-agent-status-bar-subsystem/proposal.md)（扫描 / 合并本波 `session_env`）。
> **工程约定（本波次）**：策略默认 **code-first**：`defaults.rs` 纯常量（改文件调试）；**不**新增 YAML 旋钮；**不**用 env 当未暴露配置面。用户面 YAML 仅既有字段（如 `api`）。真源见 [`c1880`](../archive/2026-08-04-c1880-update-responses-first-api-boundary/proposal.md)。

## Why

缓存友好要求「知道哪些字节会变」；coding agent 又需要 cwd/date。需要的是**显式标注与可测组装**，并把会话环境做成 **可被状态栏子系统发现的特殊类型**，而不是散落进 system 或普通 user 正文。

## What Changes

- 将 system 输入拆为片段标签：
  - `stable`：身份、工具策略、AGENTS.md、skills **元数据**
  - `session_env`：**状态栏族 bootstrap**（date / clock / cwd）——Env CustomMessage → user 投影；**默认不进 system**
  - `volatile` / 全栏：禁止进前缀；由 `c1895` `<agent_status_bar>` 消费
- 组装顺序与「只增不改」规则文档化；单测：同标签集合 → 稳定字节。
- 与 Assembler SSOT 对齐：避免 `instructions` 与 input 内 system **双份拷贝**。
- 为 `c1895` 留下可扫描契约：`custom_type` / XML 根 / `details.status_bar_kind` / `last_session_env`。

## Capabilities（意向）

- `agent-prompt` / 等价
- ContextPolicy 键（`date_placement` 默认 Omit；`status_bar_mode` 仍 Off，全栏 → c1895）

## Impact

- 后续做状态栏时有明确扫描入口，不会与普通 user 消息混淆。
- 多 agent 可只改 prompt 模板与测试，不碰 ReAct 核心。

## Out of scope

- 完整状态栏 Lane / profile / 每轮尾插（→ `c1895`）
- Compact / overflow 后 ensure session_env（→ [`c1906`](../c1906-ensure-session-env-after-compaction/proposal.md)）
- 全栏压缩保留策略（→ `c1897`）
- tool_search（→ `c1960`；**非**已归档的 `c1900` 冻表）
- 把 pwd 写回 system（否决；异 cwd resume 仍可能）

## Parallel / depends

- **硬依赖**：`c1890`
- **下游**：`c1895`（扫描 session_env）；**[`c1906`](../c1906-ensure-session-env-after-compaction/proposal.md)**（compact/overflow 保证）；软相关 `c1897`

## Open Questions

> **D1–D8 已钉入 [`design.md`](./design.md)**（session_env = 状态栏族；compact → c1906）。

- **skills 元数据**：stable（会话）；与 `$skill` user 投影对齐。
- **日历日 / cwd**：默认不进 system；`session_env` bootstrap；活时刻权威可并入 `c1895` 全栏。
- **Compact**：早期 env 可被 cut；下轮 run 会再插；overflow 同轮缺口 → `c1906`。

## Further Notes

一手深挖 → [`research/stable-volatile-split-2026.md`](./research/stable-volatile-split-2026.md)。实现后产品钉板：Omit + session_env Env→user；手测 SID 验证同 session 仅首条 env（同日同 cwd）。

## Ethics

- risk_level: low
- prohibited_actions: 无标注的隐式动态 system 注入
- required_evidence: 片段标签 + session_env 追加单测 + c1895 可发现指针
- refusal_contract: 不把「零动态 system」写成 MUST
- escalation_policy: 无
