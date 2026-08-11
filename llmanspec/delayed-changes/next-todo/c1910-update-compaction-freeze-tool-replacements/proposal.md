---
depends_on:
  - c1890-add-responses-context-policy-assembler
  - c1930-update-session-provider-view-contract
---

# 压缩：工具结果替换串首次冻结
> **一句话**：压缩生成的大工具输出替换串首次冻结，重启会话仍用同一串，保前缀一致与可复现
> **当前排序**：#7（2026-08-10 自 delayed-changes 升格入 active）


> **已升格（2026-08-10）**：自 delayed-changes 移入 active 待处理队列，当前排序 **#7**。


> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md) §6（术语对照 §7）
> **书指针**：《深入理解 AI Agent》Ch2「上下文压缩策略」（姊妹仓 `ai-agent-book/book/chapter2.md`）；书语仅经 research §7 术语表映射，**禁止**写入 live specs。
> **自包含**：在已有 auto-compact 之上钉「冻结替换」与「不砍前缀」；不做 cache 命中优化本身。冻结表归属 Session SSOT（`c1930`）。
> **工程约定（本波次）**：策略默认 **code-first**：`defaults.rs` 纯常量（改文件调试）；**不**新增 YAML 旋钮；**不**用 env 当未暴露配置面。用户面 YAML 仅既有字段（如 `api`）。真源见 [`c1880`](../archive/2026-08-04-c1880-update-responses-first-api-boundary/proposal.md)。

## Why

压缩若每次生成不同摘要文案，或从头部滑动删除消息，会破坏前缀一致性与可复现性。Claude Code 类实践：大工具输出替换串**首次冻结**，重启会话仍用同一字符串。

## What Changes

- 工具结果（及同类大块）压缩/截断替换：同一逻辑键（如 tool_call_id + 策略档）→ **首次**生成的替换串持久化，后续重放复用。
- ContextPolicy / compaction 设置：明确禁止「滑动窗口删头部」作为主策略。
- 稳定前缀（system/tools 稳定子集）不参与 compact。
- 测试：同一会话两次投影，冻结串字节一致；策略档变更有显式世代/说明。

## Capabilities（意向）

- `domain-compaction`
- 会话持久化字段（若需存冻结表）

## Impact

- 长会话 + 重启后 Prompt Cache / 前缀行为更可预期。
- 与「不追命中率」一致：目标是可复现与少意外失效。

## Out of scope

- 动态压缩档产品 UX 全集（roadmap M3 可后续切）
- Eval 成功率闸（后置）
- status bar 注入（→ delayed `c1895`）；压缩时 status 消息保留策略（→ delayed `c1897`）
- tool_search

## Parallel / depends

- **硬依赖**：`c1890`（布局边界：compact 只动轨迹）、`c1930`（冻结表在 SSOT 的位置）
- 与同层并行时注意 compaction 模块所有权

## Open Questions

- 冻结表存在 session JSONL 旁路文件还是 entry 元数据—— propose 时钉。

## Ethics

- risk_level: low
- prohibited_actions: 滑动窗口砍头部作默认；压缩伪造「未压缩」provenance
- required_evidence: 冻结串一致性测
- refusal_contract: 不承诺压缩必降成本
- escalation_policy: 无
