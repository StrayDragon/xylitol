---
change_id: c1535-optimize-tui-stream-wrap-tail
title: TUI 流式尾 wrap / scroll_render 再优化（候补）
status: purpose-draft
priority: 1535
apply_band: P9-deferred
depends_on:
  - c1509-optimize-package-tui-wrap-text-ansi
  - c1510-fix-tui-streaming-assistant-paint
author: agent
---

# c1535-optimize-tui-stream-wrap-tail

> **⚠️ P9 deferred（`llmanspec/do-not-read-me/`）**
> A+B（finalize 行复用）落地后，体感与 samply 均已明显好转。下一份额热点是 **wrap / 流式尾 scroll_render**，但是**必要画图成本**为主，ROI 暂不值得打断主线（OTEL / Langfuse 树优先）。

## Why（证据）

探针：`E-hist-stream` long-stream（流盖满 ~20s）。

| run | main samples | width | wrap | scroll_render | do_render |
|---|---|---|---|---|---|
| pre-A+B 80 | 1331 | 64.1% | 15.9% | 27.3% | 86.9% |
| **post-A+B 80** | **636** | **36.8%** | **33.0%** | **52.4%** | **75.9%** |
| pre-A+B 800 | 1294 | 66.2% | 20.4% | 30.3% | 89.1% |
| **post-A+B 800** | **640** | **37.8%** | **36.4%** | **56.6%** | **75.9%** |

解读：

1. A+B 把引擎 invariant width 砍掉一大块；主线程样本约减半。
2. wrap / scroll_render **占比上升**主要是「去掉 width 后的份额上浮」；绝对样本未爆炸。
3. 80→800 历史放大：wrap / scroll 份额几乎不动 → 仍是**流式尾重绘**，不是长历史上半 O(n) flatten（见已延后的 c1505）。
4. c1509（wrap ANSI+ASCII 快路径）与 c1510（streaming assistant 增量 paint）**已落地**；余下多是 CJK grapheme wrap + 尾块 Markdown 真画。

## ROI 评估

| 候补方向 | 体验风险 | 预估收益 | ROI |
|---|---|---|---|
| wrap CJK 再挖（少 grapheme / 更快量宽） | 低–中（emoji/ZWJ） | 中（流式 CJK 时） | 中，可候补 |
| 流式尾更激进增量（更大稳定前缀） | 中（错字/闪） | 中 | 中，需真会话复现卡顿再开 |
| 更狠合帧 / 降流式帧率 | **高**（观感一顿） | 不明确 | **低** |
| c1505 viewport slice | — | 已证非瓶颈 | **不做** |

**结论（2026-07-23）**：暂**不是**产品瓶颈；用户手动测亦「非常好」。本 change **仅 draft 延后**，不 promote / 不 apply。主线改投 **c1495 OTEL session 父子树 / Langfuse**。

升格条件（任一）：

- 真会长回复流式再次卡 spinner / 掉帧，且 E/C profile 显示 wrap 或 streaming paint 绝对成本主导；或
- 业务空窗且有明确微基准（wrap 行长 × 帧）可验收。

## 意向方案（升格时）

1. 针对 `wrap_text_with_ansi` CJK 路径再测 + 可选快路径（无 ZWJ 时）
2. 复查 c1510 稳定前缀边界；长工具/代码块流式是否仍全量 wrap
3. harness：流式尾 `finalize_width_checks` / streaming parse 计数上界（已有部分 ath）

## Out of scope

- c1505 viewport；c1370 热缓冲封顶
- 为 CPU% 牺牲流式可读性

## Status

**purpose-draft · P9-deferred** — 记档候补；主线不跟。

## Ethics

- risk_level: low（延后稿）
- prohibited_actions: 未复现卡顿就合帧伤害流式体验
