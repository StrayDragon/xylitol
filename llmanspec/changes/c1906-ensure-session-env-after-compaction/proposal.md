---
depends_on:
  - c1905-update-system-prompt-stable-volatile-split
---

# Compact / overflow 后保证 session_env

> **一句话**：压缩裁掉或同轮 overflow reload 后，LLM 上下文 MUST 仍有校正后的 session_env（状态栏族 bootstrap）
> **sourced_from**：[`c1905`](../c1905-update-system-prompt-stable-volatile-split/design.md) compact 缺口分析（2026-08-10）
> **与 c1897**：本 change 只钉 **已落地** 的 `session_env`；全栏 `AgentStatusBar` 堆积策略仍归 [`c1897`](../c1897-update-compaction-status-bar-messages/proposal.md)

## Why

`c1905` 把 date/clock/cwd 做成状态栏族 **`session_env`**（Env→user），system 默认 Omit。

压缩经 `build_context_entries` 只保留 compaction 摘要 + `first_kept` 之后：会话头附近的 `session_env` **常被裁掉**。

| 路径 | 今日行为 | 缺口 |
|---|---|---|
| 下一完整 `run`（新用户输入） | `should_append_session_env` 见空 → 再插 | 正确，但偏晚 |
| overflow compact **同轮重试** | 只 reload `build_context_entries`，**不**再跑 env 注入 | 该轮后续 model call 可能无 cwd/date |
| 多条历史 env（跨日/换目录） | cut 后可能零条或留旧条 | 需「取最新 + 相对当前 cwd/date 校正」 |

**否决**：把 pwd 写回 system first（前提「不可 load 异 cwd session」不成立——TUI `SessionScope::All` 可恢复；且双真源回潮 c1905）。

## What Changes

- **不变量**：凡进入 provider 的 history（含 overflow reload 后），在真实 user / 继续生成之前，MUST 能解析到与 **当前进程 cwd + 当前日历日** 一致的最新 `session_env`（否则追加一条校正行；append-only，不删旧 transcript 行——旧行可已在 cut 外）。
- **缝**：
  1. `compact_session` 完成后（或写完 CompactionEntry 后）可选：若 leaf 上下文已无有效 env，立即 persist 一条校正 `session_env`（减少空窗）；
  2. overflow / 任意 `build_context_entries` → history 重建后：**共用** `ensure_session_env_in_history`（或等价），与 ReAct 首轮注入同规则。
- **多条**：扫描用既有 `last_session_env`；比较用 `should_append_session_env`；实现可抽 `ensure_…(history, cwd) -> (history, did_append)`。
- **单测**：fixture 含早期 session_env + compact cut → 上下文无 env → ensure 后有一条且 cwd/date 正确；overflow reload 路径同断言。
- **文档**：c1905 / c1895 / c1897 交叉引用；agent 可读发现表不丢。

## Capabilities（意向）

- `agent-prompt` / `agent-runtime`（session_env × compact 不变量；单测为主，`feature: false` 可接受）
- 软触 `domain-compaction`（钩子，不改摘要算法）

## Out of scope

- 把 cwd/date 写回 system（否决）
- 全栏 `AgentStatusBar` keep-latest / drop-all（→ `c1897`）
- StatusBar Lane Runtime（→ `c1895`）
- 改变 TUI `SessionScope::All` 可恢复异 cwd 的产品语义

## Parallel / depends

- **硬依赖**：`c1905`（须先归档或本树已含其行为）
- **软相关**：`c1897`（全栏压缩策略；本 change 可先落地 bootstrap）
- **软相关**：`c1895`（扫描 `session_env` 时假定 compact 后仍可得）

## Open Questions

### 已钉（随 c1905 分析）

- **不写回 system pwd**。
- **异 cwd resume 允许**（非硬禁）→ env 追加校正，不改 system。
- **与 c1897 分工**：bootstrap=`session_env` 本 change；全栏堆积=c1897。

### 待 design 钉

- compact **完成时立刻 persist** env，还是仅在「下次组装 history」ensure？（倾向：overflow 路径必须 ensure；persist 可一并做以免空窗）
- ensure 是否写 store，还是仅内存 history？（overflow 重试若只改内存，下次 run 仍靠 should_append——两处都要测）

## Ethics

- risk_level: low
- prohibited_actions: 回潮 system cwd；无标记误伤普通 user；删 transcript 旧 env 破坏前缀（除非显式 c1897 策略）
- required_evidence: compact / overflow fixture 单测
- refusal_contract: 不把「session 只能同 cwd」写成硬约束
- escalation_policy: 若改回 system 含 pwd，须产品确认
