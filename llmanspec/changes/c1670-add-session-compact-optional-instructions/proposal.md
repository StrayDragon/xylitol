---
change_id: c1670-add-session-compact-optional-instructions
title: /session-compact 可选 instructions（推翻 A05）
status: purpose-draft
priority: 1670
depends_on:
  - c1640-add-compaction-turn-auto-trigger
author: agent
---

# c1670-add-session-compact-optional-instructions

> **流程**：仅 `purpose-draft`；禁止提前改 live specs。
> **依赖**：c1640（**force** manual）归档后 apply——instructions MUST NOT 落在 maybe 闸上。
> **对照**：pi `compact(customInstructions?)`；xylitol 命令名仍遵守 **A03** `session-*`。

## Why

A05 曾禁止 `/session-compact` 带参。产品改跟进 pi：**可选** instructions 聚焦摘要。

## 需求锁定

### R1 — Slash（已决）

- `/session-compact` → force compact，无 instructions。
- `/session-compact <text…>` → force compact + instructions = 去壳后文本。
- 仅空白 / 空字符串 → **视为无 instructions**（与无参同）。
- MUST NOT 再对带参报 `usage: ... custom instructions not supported`。
- 短名 `/compact` **仍然无效**（A03 不回退）。

### R2 — 传输（已决）

- `Command::Compact` / Driver force API MUST 携带 `Option<String> instructions`（或等价）。
- Auto（threshold / overflow）MUST **不传** instructions，MUST NOT 复用上一次 manual 的 instructions。

### R3 — 摘要（已决）

- `generate_summary` / compact 编排在存在 instructions 时 MUST 注入 prompt（对齐 pi `customInstructions` 拼法：附加聚焦说明，不替换整份结构化骨架，除非 pi 行为另行证明）。
- Fake/捕获测试 MUST 能证明 instructions 进入模型输入。

### R4 — 产品台账

- `src/app/tui/PI_DELTAS.md` **A05**：改为已撤销 / 已对齐，并记变更记录。
- harness：删除「带参必 usage 错误」断言；改为有参成功路径 + 空白当无参。

### R5 — 非目标

| 禁止 | 说明 |
|---|---|
| 恢复 `/compact` 短名 | A03 |
| extension 整段替换 compaction | 不做 |
| TUI % | c1680 |
| auto 路径带 instructions | 禁止 |

## 验收锚点

| id | Then |
|---|---|
| bare-force | 无参 → force compact（依赖 c1640） |
| with-text | 有参 → 摘要 prompt 含该文本 |
| whitespace | 仅空格 → 等同无参 |
| auto-clean | threshold/overflow auto 输入无 instructions |
| a05-doc | PI_DELTAS A05 已更新 |

## Capabilities

`app-tui-commands` · `domain-compaction` · driver / wire

## Open Questions

- （已决）可选；A03；空白=无。
- wire/RPC 版本字段：若有多端兼容需求，promote 时 design 一笔。

## Ethics

- risk_level: low
- prohibited_actions: 超长 instructions 明文乱入无关日志；auto 误用 manual instructions；提前改 live specs
- required_evidence: harness 三路径（无参/有参/空白）
- refusal_contract: 不做 extension confirm UI
- escalation_policy: 协议兼容争议先 design
