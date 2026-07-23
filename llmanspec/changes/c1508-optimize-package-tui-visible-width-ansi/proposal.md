---
change_id: c1508-optimize-package-tui-visible-width-ansi
title: package-tui：visible_width ANSI 快路径（免整串 strip 分配）
status: applied
priority: 1508
depends_on: []
author: agent
---

# c1508-optimize-package-tui-visible-width-ansi

> Promoted → applied（无 MUST/SHALL；quick 落地）。验收：`just test-tui` + `utils_test` visible_width。

## Why

`suite-20260723-122217`（c1520 Fake A–D）主线程栈显示：带 ANSI 的行无法走现有「纯 ASCII」快路径，每次 `visible_width` / 宽度不变量检查都走 **`strip_ansi_codes` 分配新 `String` + grapheme**。

| 场景 | width 相关栈占比（主线程） | 说明 |
|---|---|---|
| C-stream | ~59% | `visible_width`/`strip_ansi` ← `TUI::do_render`；部分经 `scrollback::{fit,markdown}` |
| D-resume | ~33% | `format_session_row_body` / `truncate_to_width` + do_render |
| B-scroll | ~30% | 同上，偏 do_render |
| A-idle | ~26% | 基线仍有不变量扫描，但绝对 samples 低 |

外部可观测宽度语义不变 → **实现/性能**；正式落地可用 **quick**（不改 MUST/SHALL），本 draft 只锁定意图与验收。

## 意向 What Changes

1. `packages/xylitol-tui/src/utils.rs`：`visible_width`（及必要时 `truncate_to_width` 共用逻辑）
   - **ANSI + 其余皆 ASCII printable**：跳过 ESC 序列时直接累加列宽，**不**分配 cleaned `String`，**不**走 grapheme
   - 含非 ASCII 时：仍可 **就地**跳过 ANSI 再 grapheme，避免中间 `String`（若基准证明值得）
2. 单测：ANSI 着色 ASCII / 混 CJK / OSC/APC 与现行为逐字宽相等
3. 回归：`just test-tui`；可选再跑 `profile-suite` C/D 对比 width 栈占比

## Out of scope

- c1505 viewport slice（见下「与 c1505」）
- 改差分引擎 API / 去掉宽度不变量
- 缓存「每行 width」（另案）

## 与 c1505

栈证据：**优先本 change**。C-stream 上 scrollback markdown/fit 约占 ~32%，viewport slice 仍可能帮长历史，但当前热点更像 **每帧对已生成行做 `visible_width`**（含不变量），不是「未切片 flatten」独占。

建议：`c1505` promote/apply 前先落地本快路径并复测 B/C。

## Status

**applied** — `visible_width_ansi_ascii` + 共用 `ansi_escape_len`；慢路径仍 strip+grapheme（tab/CJK/emoji）。

## Ethics

- risk_level: low
- prohibited_actions: 改变可见列宽语义；跳过非 ASCII 的错误快路径；把 samply 噪声进程当热点
