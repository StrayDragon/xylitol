---
change_id: c1680-add-tui-compaction-percent-display
title: TUI 上下文占用百分比派生展示
status: designed
priority: 1680
depends_on:
- c1630-update-compaction-reserve-formula
- c1670-add-session-compact-optional-instructions
author: agent
branch: sdd/c1680-add-tui-compaction-percent-display
base_sha: 8d1e8ba37a96b2a4d0d8375c071628224dcedddc
checkpointed: true
checkpoint_sha: 8d1e8ba37a96b2a4d0d8375c071628224dcedddc
---

# c1680-add-tui-compaction-percent-display

> **依赖**：c1630（reserve 触发）与 compaction 链至 c1670 均已归档；本 change **只改展示**。
> **对照**：pi footer `getContextUsage().percent` → `{p.toFixed(1)}%/{formatTokens(window)}`；xylitol 保留 provenance `used …`。
> **设计**：见同目录 `design.md` / `tasks.md`。

## Why

触发 SSOT 已是 `tokens > window − reserve`（c1630）。Footer 现有 `used N|~N|? tokens`，缺与 pi 同构的 **派生占用比**；该读数 **绝不是** 触发闸（domain-compaction c16 已预告）。

## What Changes

- **派生**：`percent = tokens / context_window * 100`（1 位小数）；`context_window` 取当前模型窗（Driver `current_model` / 等价），`tokens` 与既有 footer 同源 `ContextTokenEstimate`。
- **文案**：在既有 `used … tokens` 后追加 ` · {p}%/{fmt(window)}`；Heuristic 对百分比亦加 `~`（`~12.5%/128k`）；Unknown / window≤0 时不追加 %（或 `?%` 仅当 Unknown 且 window>0）。
- **隔离**：MUST NOT 恢复 `compaction_threshold`；MUST NOT 用 % 改 `should_compact`。
- **设计 SSOT**：更新 `design/footer.md` + atc2/atc13（或新 atc）。

## Capabilities

`app-tui-chrome` ·（只读引用）`domain-compaction` c16

## Impact

- Footer 更易读占用；触发行为不变。
- harness 需覆盖：有窗派生 / 无窗不派生 / Heuristic 波浪号 / 触发公式未改。

## Ethics

- risk_level: low
- prohibited_actions: % 写回触发；伪造 Api；恢复百分比闸配置；cache-hit 同屏叙事
- required_evidence: derived-only / no-threshold-config / trigger-unchanged / heuristic-tilde
- refusal_contract: 不做距 reserve 余量为主读数；不做 status 多行 % 条
- escalation_policy: 色阶（70/90）与 `(auto)` 指示若拉长 footer 可降为 follow-up
