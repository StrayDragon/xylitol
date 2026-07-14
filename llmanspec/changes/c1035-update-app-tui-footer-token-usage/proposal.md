---
change_id: c1035-update-app-tui-footer-token-usage
title: "Footer：used N tokens（provenance 诚实标注）"
status: full
priority: 1035
depends_on:
  - "c1030-add-package-ai-bridge"
author: agent
track: A
---

# c1035-update-app-tui-footer-token-usage

> 吸收并取代原 c655 调研成果。依赖 **c1030**（已归档）提供的 `Driver::estimate_context_tokens` + `TokenProvenance`。

## Why

原 c655 因无可信 token 数据源暂停。c1030 落地计量与 Driver 只读 seam 后，footer 可以诚实展示 `used N` / `~N` / `?`，并随 session tree travel 换叶更新。

## Purpose

1. 经 `Driver::estimate_context_tokens` 取当前 leaf 路径的 `ContextTokenEstimate`。
2. Footer 文案按 provenance：`Api` / `RemoteCount` / `LocalTokenizer` → `used N tokens`；`Heuristic` → `used ~N tokens`；`Unknown` → `used ? tokens`（或省略该字段，二选一以 design 为准：默认 **显示 `?`**）。
3. travel / turn 结束 / compact 后刷新；无估计数据时 **MUST NOT** 伪造精确 `used 0`。
4. harness：空会话 / 有消息 / travel 换叶 / Heuristic 显示 `~`。
5. 生成中活估计（可选任务）：节流刷新，**MUST NOT** 每 TextDelta 全量 encode。

## What Changes

- `format_footer_text` / UiRoot footer：加入 context token 字段
- host/effects：在适当时机调用 `Driver::estimate_context_tokens` 写入 UiModel
- harness 覆盖 provenance 文案
- delta：`app-tui-chrome`（modify footer 合约）

## Capabilities

- `app-tui-chrome`（modify）

## Out of scope

- 费用 ↑↓ / cache 分项 → **c1055**
- accounting 优先级实现 → **c1030**（已归档）

## Ethics

- risk_level: medium（产品诚实性）
- prohibited_actions: 把 Heuristic 显示成无 `~` 的精确 `used N`；无数据时伪造精确值
- required_evidence: provenance 文案对照 + travel harness
- escalation_policy: 若 Driver estimate 在 Remote 面不可用，升级确认是否仅 InProcess 显示

## Depends

- **c1030-add-package-ai-bridge**（已归档）
