---
change_id: c1505-add-tui-scrollback-viewport-slice
title: TUI scrollback：entry 级 viewport slice（长历史）
status: purpose-draft
priority: 1505
depends_on:
  - c1500-fix-tui-scrollback-perf
  - c1508-optimize-package-tui-visible-width-ansi
  - c1509-optimize-package-tui-wrap-text-ansi
author: agent
---

# c1505-add-tui-scrollback-viewport-slice

## Why

c1500 已用 `ScrollbackPaintCache`（entry fingerprint）压住「每帧全量 Markdown」的主卡顿。长会话下仍可能线性涨的是：**每帧把全部 entry 行 flatten 进 upper**，再交给差分引擎。引擎 `previous_viewport_top` 只省写屏，不省 `render_scrollback` CPU。

代码事实（`render_scrollback`）：对 `model.entries` **全量**循环；cache hit 仍 `lines.extend(cached)`——总行数随历史 **O(n) 克隆进输出 Vec**，与是否重 paint 无关。

产品面已对齐 **pi**（live 进 scrollback、不做 Codex TranscriptView）。需要在**不改产品心智**的前提下，为「历史行数 × 宽变化」留一条可验证的下一步。

## 评估（c1508 / c1509 之后）

| 项 | 结论 |
|---|---|
| c1508 / c1509 | 已 applied 并归档（`archive/2026-07-23-c1508-*` / `c1509-*`） |
| `post-c1508` C | `visible_width`/`strip_ansi` 降；剩 `wrap`/`markdown`/`scrollback` 链 |
| `post-c1509` B/C | 见下 **T0** |
| 微优化 vs 本 change | 流式**尾块**仍须真渲染；**长历史上半**的 flatten/extend 才是本 change 的主收益面 |
| 是否仍值得 | **是**（代码 O(n) extend 仍在）；短 B 种子**不足以**用 samply 证明 flatten 热 |
| promote / apply 闸 | 放大 B 种子再采 **或** 直接用 harness 行数上界落地（见 tasks） |

## T0：`post-c1509`（B+C，15s）

产物：`target/profile/post-c1509/{B-scroll,C-stream}.{json.gz,summary.txt}`

主线程栈归因（batch addr2line）：

| 场景 | main samples | scroll_render | wrap | width（含 do_render） | markdown |
|---|---|---|---|---|---|
| **B-scroll** | 71 | **0%** | 0% | ~18% | 0% |
| **C-stream** | 134 | **~27%** | **~13%** | ~61% | ~20% |

解读：

1. **B（suite 默认 ~80 pairs）**：paint cache 生效，栈上几乎不见 `render_scrollback`；短窗滚动**测不出**全量 flatten。要证据需 **更大种子**（建议 ≥300–500 pairs）或改测「输出行数 vs viewport」而非 CPU%。
2. **C**：仍有 scrollback/markdown/fit（~27% / ~20%）；`wrap` 相对 `post-c1508` 的 ~17–19% **略降到 ~13%**（c1509 有帮助）。流式尾必须真画——viewport 对 C 是次要收益。
3. **代码事实未变**：`render_scrollback` cache hit 仍全量 `extend`——结构债在；T0 结论 = **不阻塞 design**，但 **apply 前要么放大 B，要么用 harness 行数上界当验收**（不必死等火焰图）。

## 证据闸（历史）

- c1520 `suite-20260723-122217` C：width ~59%；scrollback 相关 ~32%（修前）。
- `post-c1508`：热点迁到 `wrap_text_with_ansi` ~19%（流式）；架构 flatten 问题未消。

## 别人怎么做（对照）

| 路径 | 做法 | 对我们的含义 |
|---|---|---|
| **pi** | 全量组件 `render(width)` + 差分写屏；块内 preview 裁剪；历史靠终端 scrollback | 继续同源；**主路径无** transcript 虚拟列表 |
| **Codex** | live 仅 `active_cell`；提交后 `insert_history`；Ctrl+T `PagerView` 做 viewport slice | **不宜**整套搬；slice **算法**可参考 |
| **ratatui VirtualList** | Codex 也未用 | 与 `Vec<String>` + pi 差分割裂，不优先 |

## 意向方案（推荐 A）

**A — entry 级 viewport slice（推荐）**

1. 维护 entry → 已 paint 行数（复用 `ScrollbackPaintCache` 行数，**不必估高**）
2. 在 follow-bottom（常态）时：`offset = max(0, total_lines - viewport_h - overscan)`
3. `render_scrollback` / `UiRoot::render` 只 flatten 落在窗口内的 entries（参考 Codex `PagerView::render_content` 的 skip/break）
4. 流式贴底：窗口尾部 + streaming tails 始终真渲染，避免空尾巴

**B — 估高虚拟列表**：不优先。
**C — 仅引擎 clip**：不够。

## What Changes（意向；见 design/tasks）

1. 产品 host 引入可选 `scroll_line_offset`（或等价）；默认 follow-bottom 行为与今日一致
2. `render_scrollback` 按 offset + height + margin 切片 flatten（跳过屏外 entry 的 extend）
3. harness：长 scrollback 帧耗时 / 输出行数上限断言
4. **不**引入 Codex `insert_history`；**不**做 ratatui VirtualList

## UX / 体验风险

- 快滚进未 paint 区：短暂空白或补 paint（overscan 缓解）
- fold/展开改变总高：失效窗口边界
- ath/att / PTY：可见内容不得缺字

## Out of scope

- c1495 OTEL；流式 assistant 稳定前缀增量；改信任 / 工具策略

## Status

**purpose-draft（已推进 + T0 已跑）**：依赖微优化已归档；design/tasks 已有。
**T0**：短 B 无 scroll_render 火焰；C 仍见 scrollback/markdown。apply 可用 harness 行数上界，或先放大 B 种子再采。

## Ethics

- risk_level: low（设计稿）
- prohibited_actions: 为性能切到 Codex TranscriptView；在未测基线前大改滚动语义
