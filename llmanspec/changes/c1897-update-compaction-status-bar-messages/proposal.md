---
depends_on:
  - c1895-add-agent-status-bar-subsystem
  - c1910-update-compaction-freeze-tool-replacements
---

# 压缩 × StatusBar 特殊消息（保留策略后研）

> **来源**：深挖 [`c1895`](../c1895-add-agent-status-bar-subsystem/proposal.md) Q2（2026-08-05）——Runtime 默认 **盲目尾插 `append`**；Q3 钉 **status 全部持久进 transcript**（为保住跨请求 KV / Prompt Cache）。长会话堆积后，压缩如何处理特殊标记 status 需独立调研；**本草案只保存候选项，不定案**。
> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md) §3 / §6；书 Ch2 压缩与状态栏（陈旧条 vs 注意力）。
> **工程约定**：code-first；策略未钉前 **禁止**假实现进主路径。

## Why

`c1895` 的 `append` = **不**扫描已有 status、每轮末尾直接追加。长会话 + 自动 compact 时，若不识别这些特殊消息：

- 摘要可能把陈旧/最新 status 搅进散文，毁掉「键值权威」；
- 或整段删光导致模型失去唯一新鲜读数；
- 或保留全部 status 浪费 token 且注意力被旧条稀释。

需要压缩路径上的 **显式 status 策略**，与工具结果冻结（`c1910`）分开想清楚。

## Candidate strategies（未定；后研二选一或可配置）

| 策略 | 行为 | 直觉利弊 |
|---|---|---|
| **A · keep-latest-one** | compact 后最多保留 **一条**（通常最新）特殊标记 status | 仍有一眼可读权威；旧条清掉；实现要定义「最新」与多列（Runtime vs Agent）是否分开各留一条 |
| **B · drop-all** | compact 边界内 **不保留** status；靠下一轮盲目尾插重新注入 | 压缩结果最干净；resume/compact 当轮到下一请求前可能短暂无栏；依赖注入缝可靠 |

其它可能（仅记名，非本波偏好）：按 lane 分别策略；compact 后强制立即 append 一条再继续。

## What Changes（意向；调研后再切 Designed）

- 识别契约：session/transcript 上 status 为 **独立 entry kind**（`c1895` Q7）；投影层可再包 `<agent_status>`——compact **认 kind**，不靠扫正文标签
- compaction 管道钩子：在 cut / summarize 时对标记消息走 A 或 B（或可切换）
- 与 `c1910` 冻结替换正交：status 不是工具结果替换串
- 测试缝：假轨迹含 N 条 StatusBar kind → compact → 断言条数/内容符合所选策略

## Explicitly deferred

- A vs B 最终选型与是否 YAML/code-first 双档
- Agent 列（`c1896`）message 是否与 Runtime 共用同一标记命名空间
- 压缩摘要正文是否允许引用 status 键值（倾向：否，避免散文权威）

## Capabilities（意向）

- `domain-compaction`（status 分支）
- 软依赖 `agent-*` status 标记定义（来自 `c1895`）

## Out of scope

- Runtime 注入与默认 append（→ `c1895`）
- Agent 列 TODO 业务（→ `c1896`）
- 工具结果首次冻结（→ `c1910`）

## Parallel / depends

- **硬依赖**：`c1895`（标记与注入先立）、`c1910`（压缩冻结骨架；可并行调研但 apply 宜在其后或同波协调）
- 软相关：`c1930`（持久投影）

## Open Questions

- A vs B：长 coding 轨迹更怕「无栏空窗」还是「旧条误导」？（后研 + 可选消融）
- Runtime / Agent 两列 compact 时是否 **各** keep-latest-one？
- drop-all 时是否在 compact 完成事件后 **同步** 强制 append 一条，消除空窗？

## Ethics

- risk_level: medium（压缩误删/误留会改变模型信任面）
- prohibited_actions: 把 status 散文进摘要当权威；无标记误伤普通 user 消息
- required_evidence: 选定策略后有可重复 compact fixture
- refusal_contract: 不宣称某保留策略普遍更优，须对照后再钉默认
- escalation_policy: 改变默认保留策略须产品确认

## Provenance

| 字段 | 值 |
|---|---|
| sourced_from | `c1895-add-agent-status-bar-subsystem`（深挖 Q2 append + Q3 持久 transcript） |
| captured | 2026-08-05 |
| status | purpose-draft（想法保险箱；策略 A/B 未定） |
