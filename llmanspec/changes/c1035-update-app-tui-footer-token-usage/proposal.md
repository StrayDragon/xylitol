---
change_id: c1035-update-app-tui-footer-token-usage
title: "Footer：used N tokens（provenance 诚实标注）"
status: purpose-draft
priority: 1035
depends_on:
  - "c1030-add-package-ai-bridge"
author: agent
track: A
---

# c1035-update-app-tui-footer-token-usage

> **status: purpose-draft** — 待 c1030 归档后 promote 为 full（specs+tasks）再 apply。
> 吸收并取代原 c655 调研成果。

## Why

原 c655 因无可信 token 数据源暂停。c1030 落地 `ContextTokenEstimate` + `TokenProvenance` 与 Driver 预留 seam 后，footer 可以诚实展示 `used N` / `~N` / `?`，并随 session tree travel 换叶更新。

## Purpose

1. 经 Driver 只读 API 取当前 leaf 路径的 `ContextTokenEstimate`。
2. Footer 文案按 provenance：`Api`/`RemoteCount`/`LocalTokenizer` → `used N`；`Heuristic` → `used ~N`；`Unknown` → `used ?`。
3. travel / turn / compact 后刷新；harness 覆盖空会话 / 有消息 / travel 换叶。
4. 可选同 change 或紧随任务：生成中节流活估计（非每 delta 全量 encode）。

## Capabilities（promote 时）

- `app-tui-chrome`（modify footer）
- Driver 只读 Estimate seam（若 c1030 已留形状则接线）

## Out of scope

- 费用 ↑↓ / cache 率（见 c1055）
- 再实现 accounting 优先级（属 c1030）

## Ethics

- risk_level: medium（产品诚实性）
- prohibited_actions: 把 Heuristic 显示成无 `~` 的精确值
- required_evidence: provenance 文案对照 + travel harness

## Depends

- **c1030-add-package-ai-bridge**（MUST 已归档才可 apply）
