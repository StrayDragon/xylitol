---
depends_on: []
title: AO 滚动跟手性：对齐 pi / opencode 的丝滑度
---

## Why

用户实感：ApplicationOwned 会话滚轮 / 拖动滚动**卡、不跟手**，对比 pi 与 opencode 同规模 transcript 明显更钝（无帧率数据，体感级）。卡顿点未定位——可能嫌疑：每 notch 全组件重排 vs reproject 快路径未命中、`ao_wheel_render_requested` 节流/合流策略、`paint_visible` 后 finalize 全行 `visible_width` 重算、diff paint 行级 reuse 失效导致大面积重绘、或输入事件合帧缺失（pi 有 frame-coalesce）。

既有观测基建：`src/app/tui/lab_ao_perf.rs` 已测 `wheel_paint p50/p95` 与 `ao_reproject_frames` 比率——调研从它起步，先量化「卡」在哪一段（输入→render 请求→do_render 分支→finalize→diff paint），再谈修。

## What Changes

- **主修复（E1）**：busy/streaming 期动画（spinner 等）tick 的 stale 粒度从「全组件树」细化到「dock/受影响组件」，消除 60Hz×全量重排（dev 18.4ms/帧）对交互帧的挤占。实测与方案见 `research/latency-findings.md`（F2/E1）。
- **诊断基建（F6）**：修 `lab_ao_stream_delta_perf_report` 陈年坏（缺 `on_run_started`→idle fence 吞 TextDelta），随本票入库；补 busy 期混合负载 lab 场景（spinner tick + wheel 交替）作为 E1 前后对比。
- **运行 profile（E2）**：确认产品 run 路径 release 编译；dev 另行提示（F3，3.6× 差距）。
- 候选（数据支持后议）：finalize reuse dev 未命中复辨（F4）；非本票——滚轮管线健康（F1）无需动。

## Further Notes

- 调研全文：`research/latency-findings.md`（lab 实测 F1–F6 + pi `loader.ts`/`tui.ts` 对照）。
- 关键数字：release wheel p50 12us（管线健康）；warm 全重排 dev 18.4ms / release 5.1ms；60Hz×18.4ms≈110% 单核 = 卡顿主嫌。
- 用户复现条件问卷（busy/idle、dev/release、终端）见 research E3——实施前须确认。

## 非目标（草案）

- 不改滚动语义（notch 步长、follow 规则不变）。
- 不引入新渲染后端 / GPU。
