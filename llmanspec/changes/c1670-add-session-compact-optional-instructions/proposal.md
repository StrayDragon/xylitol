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

## Why

产品曾刻意差异 A05：`/session-compact` **仅无参**，带参报 usage。现决定跟进 pi：`/compact [instructions]`——**instructions 可选**；有则聚焦摘要，无则默认结构化摘要。需推翻 `src/app/tui/PI_DELTAS.md` A05，并打通 Driver / summarizer。

## What Changes

- Slash：`/session-compact` 无参 = force compact；`/session-compact <text…>` = force compact + 自定义聚焦说明。
- 删除「带参 usage 错误」；更新 commands / harness / keybindings 文案。
- `Command::Compact` / `XyDriver::compact`（或扩展参数）携带 `Option<String> instructions`。
- `generate_summary` / compact 编排把 instructions 注入 prompt（对齐 pi `customInstructions` 拼法）。
- 更新 `PI_DELTAS.md`：A05 改为「已对齐 / 决议撤销」并记变更记录。
- Auto-compact（阈值/overflow）**默认不传** instructions（与 pi 一致）。
- **本 change 不做**：extension 自定义整段 compaction；改 slash 短名 `/compact`（A03 仍保留 `session-*` 前缀）。

## Capabilities

| Capability | 变更 |
|---|---|
| `app-tui-commands` | 可选 instructions 解析与 usage |
| `domain-compaction` | 摘要可接受 optional focus instructions |
| `app` driver / wire | Compact 命令携带可选字段 |

## Impact

- **破坏性**：推翻 A05；依赖「带参必报错」的 harness 断言需改。
- **默认体验**：无参行为 = force compact（依赖 c1640）；有参时摘要更可导向。
- **非目标**：恢复短名 `/compact`；TUI 百分比条。

## Depends / 后续

```text
c1640 ──► c1670 (本)
```

（需要 force 手动路径先存在，避免 instructions 落在 maybe 闸上。）

## Open Questions

- （已决）instructions **可选**。
- （已决）命令名仍为 `/session-compact`（A03）。
- 空字符串 / 仅空白：视为无 instructions。

## Ethics

- risk_level: low
- prohibited_actions: 把用户 instructions 泄漏进无关日志/trace 明文超长 dump；auto 路径误用上次 manual instructions
- required_evidence: harness：无参 force；有参进入摘要 prompt（可用 fake 捕获）；空白当无参
- refusal_contract: 不引入 pi extension confirm UI
- escalation_policy: 若 wire/RPC 兼容需要版本字段，先 design 再扩
