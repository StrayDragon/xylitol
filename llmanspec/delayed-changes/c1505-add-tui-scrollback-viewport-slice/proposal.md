---
change_id: c1505-add-tui-scrollback-viewport-slice
title: TUI scrollback：entry 级 viewport slice（长历史）
status: purpose-draft
priority: 1505
apply_band: P9-deferred
depends_on:
  - c1500-fix-tui-scrollback-perf
  - c1508-optimize-package-tui-visible-width-ansi
  - c1509-optimize-package-tui-wrap-text-ansi
author: agent
---

# c1505-add-tui-scrollback-viewport-slice

> **⚠️ P9 deferred（`llmanspec/do-not-read-me/`）**
> T0d 已复现合格探针 `E-hist-stream`（长流盖满采样窗）：`b_pairs` 80→400→800 时 inclusive `scroll_render` ≈ **27.3% / 27.7% / 30.3%**，**不随历史明显上涨**。端到端热点仍是 width / do_render / wrap。
> warm flatten 的 O(n) 结构债仍在（微基准），但**不是当前可证瓶颈** → 彻底延后；有真会话卡顿或改「行数上界」验收口径时再升格。

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
| 是否仍值得 | **结构债仍在**；**端到端 CPU 未证明**随历史线性变热（见 T0d） |
| promote / apply 闸 | **继续暂缓**。E 探针合格，但 80→800 未抬升 scroll_render 份额 |

## T0：证据收集（避免误导）

### A. `post-c1509` 默认 B/C（15s）

产物：`target/profile/post-c1509/`

| 场景 | scroll_render | wrap | 备注 |
|---|---|---|---|
| B (~80 pairs) | **0%** | 0% | 测不出 flatten |
| C | ~27% | ~13% | 流式重绘 upper |

### B. 放大 B 种子（`--b-pairs`，18s）

产物：`target/profile/b-pairs-{100,400,800}/`

| b_pairs | scroll_render | width≈ | clone_extend≈ | do_render≈ |
|---|---|---|---|---|
| 100 | **0%** | ~25% | ~7% | ~31% |
| 400 | **0%** | ~17% | ~8% | ~38% |
| 800 | **0%** | ~17% | ~6% | ~28% |

**关键更正**：PageUp/PageDown **不** bump `upper_gen` → `UiRoot` 走 `upper_cache_lines` 克隆，**不会**再进 `render_scrollback`。因此 **B-scroll 火焰图不是 c1505 的合格探针**；放大种子也不会出现 `scroll_render` 栈——这是机制，不是种子不够。

### C. 微基准（可复现）

测试：`scrollback_warm_flatten_grows_with_entry_count`（warm `render_scrollback` + clone）

实测（debug test，数量级）：

| entries | 输出行数 | ns/iter |
|---|---|---|
| 50 | 299 | ~9.1e4 |
| 200 | 1199 | ~3.6e5 |
| 400 | 2399 | ~7.6e5 |

→ warm flatten+clone **大致随行数线性涨**（50→400 ≈ 8× 行 / ≈ 8× 时间）。路径成本真实；**不等于**流式端到端热点。

### D. T0d — `E-hist-stream`（长历史预载 + 流式 bump upper）

Suite 新增场景 **`E-hist-stream`**：`--session` 预载 `b_pairs` + `XYLITOL_FAKE_SLOW_STREAM` 覆盖整段采样窗（约 `duration` 秒；短流 ~0.6s 会 idle 稀释，**不可用**）。

Inclusive 栈归因（`summarize_samply_profile.py` 新增段落）：

| run | b_pairs | main samples | scroll_render | wrap | width | do_render |
|---|---|---|---|---|---|---|
| `e-hist-long-80b` | 80 | 1331 | **27.3%** | 15.9% | 64.1% | 86.9% |
| `e-hist-long-400` | 400 | 1323 | **27.7%** | 17.2% | 65.5% | 89.7% |
| `e-hist-long-800` | 800 | 1294 | **30.3%** | 20.4% | 66.2% | 89.1% |

对照：短流 E（`e-hist-{80,400,800}`）样本过少 / idle 稀释 → **作废**，勿引用。

解读：

1. **E 是合格探针**：`scroll_render` 稳定出现（~27–30%），不像 B 恒 0%。
2. **历史放大未抬升主因**：80→800 pairs，`scroll_render` 仅 27.3→30.3（噪声级）；流式下热点仍是 **width / do_render / wrap / 流式尾**，不是「历史上半 flatten」随 n 线性爆炸。
3. 与微基准不矛盾：warm extend 有成本，但相对 Fake 长流式下的 width/wrap **不够大到成为 samply 份额主因**。
4. **因此 T0d 不支持立刻落地 c1505**——除非另有产品卡顿（真会话帧时）或换验收口径（harness 行数上界 / 纯结构债）。

### T0 结论（决策）

1. **不要**用 B-scroll samply 决定 c1505（假阴性）。
2. **要用** E-hist-stream，且流必须覆盖采样窗。
3. 结构成本（warm extend）微基准可见；**端到端尚未证明**「更长历史 → 更热的 scroll_render」。
4. **保持 purpose-draft，不 propose、不 apply**；若继续，优先查「真会话卡顿」或改测「输出行数上界」而非 CPU%。

## 证据闸（历史）

- c1520 `suite-20260723-122217` C：width ~59%；scrollback 相关 ~32%（修前）。
- `post-c1508`：热点迁到 `wrap_text_with_ansi` ~19%（流式）；催生 c1509。
- **B 放大 100/400/800：scroll_render 恒 0%**（upper_cache）。
- **E long-stream 80/400/800：scroll_render ~27–30%，不随 n 明显上涨。**

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

**purpose-draft · P9-deferred** — 已移入 `llmanspec/do-not-read-me/`。
T0d 复现：E long-stream 合格探针；历史放大未证明 scroll_render 瓶颈。

## Ethics

- risk_level: low（设计稿）
- prohibited_actions: 为性能切到 Codex TranscriptView；在未测基线前大改滚动语义
