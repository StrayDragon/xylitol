---
change_id: c1670-add-session-compact-optional-instructions
title: /session-compact 可选 instructions（推翻 A05）
status: designed
priority: 1670
depends_on:
- c1640-add-compaction-turn-auto-trigger
- c1660-add-compaction-overflow-retry
author: agent
branch: sdd/c1670-add-session-compact-optional-instructions
base_sha: 8d1e8ba37a96b2a4d0d8375c071628224dcedddc
checkpointed: true
checkpoint_sha: 8d1e8ba37a96b2a4d0d8375c071628224dcedddc
---

# c1670-add-session-compact-optional-instructions

> **依赖**：c1640（force manual）与 c1660（overflow）均已归档；instructions MUST NOT 落在 maybe/auto 闸上。
> **对照**：pi `compact(customInstructions?)` → `generateSummaryWithUsage` 追加 `Additional focus:`；**不**注入 turn-prefix 摘要。
> **命名**：仍遵守 **A03** `session-*`；短名 `/compact` 无效。
> **设计**：见同目录 `design.md` / `tasks.md`。

## Why

A05 曾禁止 `/session-compact` 带参。产品改跟进 pi：**可选** instructions 聚焦摘要，不替换结构化骨架。

## What Changes

- **Slash**：无参 / 仅空白 → force、无 instructions；非空文本 → force + instructions；MUST NOT 再报 `custom instructions not supported`。
- **传输**：`Command::Compact` / `XyDriver::compact` / `force_compact` 携带 `Option<String> instructions`（serde 默认缺省兼容）。
- **摘要**：`generate_summary` 在存在 instructions 时追加 `\n\nAdditional focus: …`；split-turn 仅 history 摘要带；turn-prefix MUST NOT。
- **Auto**：threshold / overflow MUST 不传 instructions，MUST NOT 复用上次 manual。
- **台账**：`PI_DELTAS` A05 → 已撤销/已对齐；harness 三路径（无参/有参/空白）。

## Capabilities

`domain-compaction` · `app-tui-commands`

## Impact

- 用户可聚焦压缩摘要；wire 字段向后兼容（缺省 = None）。
- A05 从「不得回退差异」改为已对齐记录。

## Ethics

- risk_level: low
- prohibited_actions: auto 误用 manual instructions；超长 instructions 明文乱入无关日志；恢复 `/compact` 短名；extension 整段替换 prompt；提前做 c1680 TUI%
- required_evidence: bare-force / with-text / whitespace / auto-clean / a05-doc
- refusal_contract: 不做 replaceInstructions / confirm UI
- escalation_policy: 远程 Driver / REST 字段争议先 design 一笔兼容
