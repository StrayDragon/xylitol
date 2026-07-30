---
change_id: c1790-fix-tui-rebuild-tool-merge
title: Resume/travel 重建 Tool 与直播单块对齐
status: designed
priority: 1790
depends_on: []
blocks: []
author: agent
branch: sdd/c1790-fix-tui-rebuild-tool-merge
base_sha: 096ef094cfed1ab7b504b854939b7f7b43d5a0c7
checkpointed: true
checkpoint_sha: 096ef094cfed1ab7b504b854939b7f7b43d5a0c7
---

# c1790 — Resume/travel 重建 Tool 与直播单块对齐

> travel / fork / resume 共用重建路径：同一次工具调用在 scrollback 中 MUST 呈现为 **一条** `UiEntry::Tool`（args + output），与直播 `ToolExecutionEnd` 后形态对齐。
> **不**改 session JSONL / LLM `project_for_llm` / `s19`（仍为 assistant toolCall + 独立 toolResult + `toolCallId`）。

## Why

大 session resume 后，Write/Grep/Find/Bash 等在 UI 上变成「调用一行 + 结果一行」两步；直播则是同 id upsert 后就地填结果 → **一块**。
根因：`rebuild_scrollback_from_travel` 逐条投影，`toolResult` 用 session `entry_id` 当 Tool id，且不读 `toolCallId`，无法与 `fc_…` 配对。
外视不一致损害信任；LLM 路径不受影响，但仍须钉死 TUI 合约，避免回归。

## 已拍板

| 项 | 决定 |
|---|---|
| 真源 | JSONL 仍双行（call + result）；UI 投影合成一块 |
| 实现倾向 | **共享投影语义（B）**：rebuild 复用 live 的 upsert / 填结果规则，而非仅 session_tree 内临时 merge |
| LLM | **零影响**；禁止为 UI 改 wire 或嵌套 toolResult |
| demo | 若 demo 有同类拆分则同对齐；无则不动 |

## What Changes

1. 收紧 `att12`（+ feature）：重建后同 `toolCallId` 的 toolCall + toolResult → **恰好一条** done Tool；幂等范围扩到 Tool（相对 live End 后）。
2. 抽取/共用「把 toolResult 合入已有 Tool 行」的投影（对齐 `ToolExecutionEnd` / `humanize_tool_result_for_tui` / display_diff）。
3. `rebuild` 累积 entries 时按路径顺序投影；处理 toolResult 时按 `toolCallId` merge；禁止再用 entry UUID 当 Tool id（有 call 可配时）。
4. 单测 + feature：grep/write 等 fixture 重建后 `UiEntry::Tool` 计数与字段（preview + output + done）。

## Capabilities

- `app-tui-transcript`（重建 / 外视幂等）
- `app-tui-bridge`（共用投影规则指针；直播 atb10 不变）

## Impact

- **用户**：resume/travel 后工具块与当场跑完一致。
- **LLM / store**：无。
- **风险**：orphan toolResult（缺 call）仍须可投影为单行 done Tool。

## 测试边界（seam）

| Seam | 用途 |
|---|---|
| `rebuild_scrollback_from_travel` + `session_entry_to_ui_entries`（`src/app/tui/bridge/session_tree`） | 主单测 / `@req:att12` feature 所驱动的公共重建边界 |
| 既有 `apply_xy_event` Tool 族（对照） | 直播单块不回归；不新造第二 harness |

MUST 复用上述边界；MUST NOT 另起脱离 feature 的 CLI。
