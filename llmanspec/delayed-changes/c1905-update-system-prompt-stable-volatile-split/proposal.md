---
depends_on:
  - c1890-add-responses-context-policy-assembler
apply_band: P9-deferred
---

# System prompt：稳定 / 可变切分（配置化，不教条）

> **⚠️ deferred（2026-08-05）**：移入 `llmanspec/delayed-changes/`，避免污染本期 SDD graph。本期实现线 = **c1920 → c1930/c1925 → c1900（MCP tool_search）**；Todo/状态栏等扩展后置。


> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md) §5–6（术语对照 §7）
> **书指针**：《深入理解 AI Agent》Ch2「KV Cache 友好的上下文设计」「提示工程」（姊妹仓 `ai-agent-book/book/chapter2.md`）；书语仅经 research §7 术语表映射，**禁止**写入 live specs。
> **自包含**：只整理 prompt 片段分类与组装顺序；不实现状态栏/MCP search。
> **工程约定（本波次）**：策略默认 **code-first**：`defaults.rs` 纯常量（改文件调试）；**不**新增 YAML 旋钮；**不**用 env 当未暴露配置面。用户面 YAML 仅既有字段（如 `api`）。真源见 [`c1880`](../archive/2026-08-04-c1880-update-responses-first-api-boundary/proposal.md)。

## Why

缓存友好要求「知道哪些字节会变」；coding agent 又允许 cwd/date 留在 system。需要的是**显式标注与可测组装**，不是强行把一切动态挪到末尾。

## What Changes

- 将 system/developer（或 `instructions`）输入拆为片段标签，例如：
  - `stable`：身份、工具策略、AGENTS.md、skills **元数据**
  - `session_env`：cwd 等（**允许**进稳定前缀；默认保留现状心智）；**date 单独标注**——单日可进前缀，但隔日 resume 有坑，选型见 Open Questions，勿与 cwd 捆死
  - `volatile`：禁止进前缀，只经状态栏或工具（由 `c1895` 消费）
- 组装顺序与「只增不改」规则文档化；单测：同标签集合 → 稳定字节。
- 与 Assembler SSOT 对齐：避免 `instructions` 与 input 内 system **双份拷贝**。
- 过长 skills/MCP 说明：优先指向渐进披露（search/skills），而非为 cache 留垃圾前缀。

## Capabilities（意向）

- `agent-prompt` / 等价
- ContextPolicy 键

## Impact

- 后续改状态栏/tools 时不会误伤「不该动」的片段。
- 多 agent 可只改 prompt 模板与测试，不碰 ReAct 核心。

## Out of scope

- 状态栏实现（→ `c1895`）
- tool_search（→ `c1900`）
- 强制迁出 date/cwd

## Parallel / depends

- **硬依赖**：`c1890`
- 与同层并行时注意 prompt 文件冲突，可用任务切分（模板 vs 测）

## Open Questions

- skills 元数据放 stable 还是末尾 meta（书倾向末尾通道）—— propose 时与现 `$skill` 行为对齐。
- **日历日 `date` × 隔日 resume（深挖必做）**：同一 session 跨自然日继续聊时，date 若钉在 system 会过时；若每日改写 system 则整段前缀失效。需在 propose/design 中对比并选型（可与 `c1895` 联调）：(a) system 内 date + 显式「日界刷新」规则与观测；(b) date 迁状态栏（replace/append）；(c) system 不含 date，仅跨日首轮追加 meta。cwd 与 date **分开**论证（cwd 通常会话内不变）。调研指针：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md) §1。

## Ethics

- risk_level: low
- prohibited_actions: 无标注的隐式动态 system 注入
- required_evidence: 片段标签 + 组装单测
- refusal_contract: 不把「零动态 system」写成 MUST
- escalation_policy: 无
