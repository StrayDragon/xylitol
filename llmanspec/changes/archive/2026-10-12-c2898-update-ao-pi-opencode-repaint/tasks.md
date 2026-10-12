# Tasks — c2898 AO 交互卡顿

前置：research/latency-findings.md（F1–F6）+ design（D1–D3）。**实施前须用户确认 E3 复现场景**（busy/idle、dev/release、终端）与 D1 未决两项。

- [x] T0 诊断基建：修 `lab_ao_stream_delta_perf_report`（on_run_started，F6）；新 `lab_ao_busy_mixed_perf`（spinner tick×wheel 交错，D2）——先跑 E1 前基线存档。
- [x] T1 E2 快刀：核查产品 run 路径编译 profile；非 release 则改默认/提示（一行级）。
- [x] T2 D1 灰度：`AoDirty` 三态 + `tick_dirty` 默认方法（全组件默认 Restale=行为不变）；`do_render` Repaint 路径（dock-only 重排）；spinner/clock 迁移 Repaint。
- [x] T3 busy mixed lab E1 前后对比：dev/release p95 达标（D3），数据回写 research。
- [x] T4 全量门禁 `just qa` + E1 后 `lab_ao_perf` 三报告不回归；收口 finalize。
