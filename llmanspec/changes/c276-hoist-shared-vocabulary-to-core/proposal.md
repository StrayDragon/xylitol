---
depends_on:
  - c275-refactor-layer-enforcement
---

# c276-hoist-shared-vocabulary-to-core

> **状态**：draft 提案（2026-06-27）。c275 路线图第一项，依赖 c275 建立的白名单（本变更逐项删除
> 类型性耦合的白名单条目）。

## Why

c260 把 provider/tools 运行时迁到了 `infra/`，但**被多个层共享的纯数据类型**也跟着落进了 infra，
逼着 `agent/` 反向 import infra 仅为取类型——这是 c275 review 发现的 38 处生产违规中约 17 处的
**结构性根因**。这些类型零运行时、零 I/O，本属 `core/` 词汇层（la1）。`core/` 当前仅 1,603 LOC
偏小，正是因为词汇在 c260 迁移时未上提。

## What Changes

把以下纯数据类型（及其依赖链）从 `infra/` 迁到 `core/`，agent 改 import 路径：

- `SessionEntry` 族（`MessageEntry`/`CompactionEntry`/`EntryBase`/`ThinkingLevelChangeEntry`/
  `ModelChangeEntry`）← `infra/session/types.rs`
- `AgentLifecycleEvent` 类型 ← `infra/event/lifecycle.rs`（EventBus 实现留 infra，c277 处理）
- `SkillInfo`/`PromptTemplate`/`ThemeInfo`/`ResourceDiagnostic` ← `infra/resource`（loader 实现留 infra）
- `SourceInfo`/`SourceOrigin`/`SourceScope` ← `infra/source_info`
- `CompactionConfig`（← `infra/config/types`）/ `CompactionSettings`（← `infra/settings/types`）
- `xml_escape` 纯函数 ← `infra/skills/loader`（跨层共享的纯字符串工具，归 core）

类型移动后，`agent/compaction/`、`agent/prompt/`、`agent/tools/definition.rs`、
`agent/session/{mod,export,events,steering}.rs` 中约 17 处 `crate::infra::` 类型 import
改为 `crate::core::`，相应从 c275 白名单删除。

## Capabilities

- `layer-architecture`（modify）：强化 la1（core 词汇层必须容纳被 ≥2 层共享的纯数据类型）

## Impact

- **白名单收缩 ~17 项**：c275 arch_guard 白名单中类型性条目随之删除，护栏自动收紧。
- **零行为变更**：纯类型迁移 + import 路径重写；BDD 不受影响。
- **infra 模块瘦身**：`infra/session/types.rs`、`infra/source_info` 等降为对 core 的薄 re-export
  或删除（若仅类型）。
