---
depends_on: []
title: AO 滚动跟手性：对齐 pi / opencode 的丝滑度
---

## Why

用户实感：ApplicationOwned 会话滚轮 / 拖动滚动**卡、不跟手**，对比 pi 与 opencode 同规模 transcript 明显更钝（无帧率数据，体感级）。卡顿点未定位——可能嫌疑：每 notch 全组件重排 vs reproject 快路径未命中、`ao_wheel_render_requested` 节流/合流策略、`paint_visible` 后 finalize 全行 `visible_width` 重算、diff paint 行级 reuse 失效导致大面积重绘、或输入事件合帧缺失（pi 有 frame-coalesce）。

既有观测基建：`src/app/tui/lab_ao_perf.rs` 已测 `wheel_paint p50/p95` 与 `ao_reproject_frames` 比率——调研从它起步，先量化「卡」在哪一段（输入→render 请求→do_render 分支→finalize→diff paint），再谈修。

## What Changes

- TODO（propose 时锁）：以 lab 实测为据，锁定热点后改造 repaint 链相应段。
- 候选方向（未拍板，仅记录）：输入侧合帧；wheel 快路径命中率；finalize 增量宽度检查；diff reuse 提升到滚动平移特例（vertical shift 已有检测，或可扩展）。
- 验收口径：与 pi/opencode 同规模 transcript 对比滚动手感；`lab_ao_perf` p95 不回归且重投影比率显著上升。

## 非目标（草案）

- 不改滚动语义（notch 步长、follow 规则不变）。
- 不引入新渲染后端 / GPU。
