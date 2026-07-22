---
change_id: c1505-add-tui-scrollback-viewport-slice
title: TUI scrollback：entry 级 viewport slice（长历史）
status: purpose-draft
priority: 1505
depends_on:
  - c1500-fix-tui-scrollback-perf
author: agent
---

# c1505-add-tui-scrollback-viewport-slice

## Why

c1500 已用 `ScrollbackPaintCache`（entry fingerprint）压住「每帧全量 Markdown」的主卡顿。长会话下仍可能线性涨的是：**每帧把全部 entry 行 flatten 进 upper**，再交给差分引擎。引擎 `previous_viewport_top` 只省写屏，不省 `render_scrollback` CPU。

产品面已对齐 **pi**（live 进 scrollback、不做 Codex TranscriptView）。需要在**不改产品心智**的前提下，为「历史行数 × 宽变化」留一条可验证的下一步。

## 别人怎么做（对照）

| 路径 | 做法 | 对我们的含义 |
|---|---|---|
| **pi** | 全量组件 `render(width)` + 差分写屏；块内 preview 裁剪；历史靠终端 scrollback | 继续同源；**主路径无** transcript 虚拟列表 |
| **Codex** | live 仅 `active_cell`；提交后 `insert_history`；Ctrl+T `PagerView` 做 viewport slice | **不宜**整套搬（与 `app/tui/AGENTS.md` 冲突）；slice **算法**可参考 |
| **ratatui VirtualList** | Codex 也未用 | 与 `Vec<String>` + pi 差分割裂，不优先 |

## 意向方案（推荐 A）

**A — entry 级 viewport slice（推荐）**

1. 维护 entry → 已 paint 行数（复用 `ScrollbackPaintCache` 行数，**不必估高**）
2. 在 follow-bottom（常态）时：`offset = max(0, total_lines - viewport_h - overscan)`
3. `render_scrollback` / `UiRoot::render` 只 flatten 落在窗口内的 entries（参考 Codex `PagerView::render_content` 的 skip/break）
4. 流式贴底：窗口尾部始终真渲染，避免空尾巴

**B — 估高虚拟列表**：Markdown/diff 行高随 width 变，维护成本高；三家均无先例 → **不优先**。

**C — 仅引擎 clip**：不省 paint CPU → **不够**。

## What Changes（仅意向；本 draft 不写代码）

1. 产品 host 引入可选 `scroll_line_offset`（或等价）；默认 follow-bottom 行为与今日一致
2. `render_scrollback` 按 offset + height + margin 切片 flatten
3. harness：长 scrollback streaming 帧耗时 / 行数上限断言（与 ath25 对齐方向）
4. **不**引入 Codex `insert_history` 分裂架构；**不**做 ratatui VirtualList

## UX / 体验风险（先讨论再 promote）

- 快滚进未 paint 区：短暂空白或补 paint 卡一下（用 overscan 缓解）
- fold/展开改变总高：需失效窗口边界
- ath/att / PTY：可见内容不得缺字；屏外可不做工作

## Out of scope

- c1495 OTEL span 树
- 流式 assistant「稳定前缀 + 尾 block」增量（可另开）
- 改信任 / 工具策略

## Status

**purpose-draft** — 有实测「与历史长度线性」的 CPU 后再 promote；当前 c1500 已压住主痛点。

## Ethics

- risk_level: low（设计稿）
- prohibited_actions: 为性能切到 Codex TranscriptView；在未测基线前大改滚动语义
