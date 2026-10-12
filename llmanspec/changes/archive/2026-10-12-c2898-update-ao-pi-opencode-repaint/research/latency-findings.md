# c2898 滚动/交互卡顿调研（lab 实测 + 对标）

日期：2026-10-12。全部数据本机实测（lab_ao_perf，120×40，571 行组件输出 / 40 行视口 / dock 2）。

## F1 滚轮链路本身健康——「滚动卡」非滚轮管线

| 指标（release） | 值 |
|---|---|
| wheel_paint p50 / p95 | **12us / 12us**，reprojected 40/40（快路径全命中） |
| idle_paint p50 | 3us |
| drag_paint p50 | 51us |
| vertical_shifted | 40/40（平移检测命中） |

dev build 同链路 wheel p50 681us——仍 <1ms。**滚轮→重投影→diff paint 管线无热点**。

## F2 主嫌疑：busy/streaming 期 spinner tick 触发全量组件重排

- `tui/mod.rs` tick：spinner/chrome 动画→`ao_components_stale = true`→**全组件树重 render**。
- 全量重排成本（warm 帧）：release **5.1ms**、dev **18.4ms**（571 行 markdown/语法高亮 rewrap）。
- 60Hz tick × 18.4ms ≈ **110% 单核**——dev 下 busy 期滚动/交互必然抢不到帧；release 30%+。
- pi 对照：loader 仅 `setText`+`requestRender`，同为全渲染模型，**但 pi 无「spinner→全局 stale」语义**（组件级 invalidate 即可重渲染？pi 靠 JS 快+树小）。我们的优化方向＝**stale 粒度细化：动画只 dirty dock/受影响组件**（dock 行重排 ~1ms 级），非全树。

## F3 dev/release 差距 3.6×——产品运行 profile 待确认

warm 18.4ms(dev) vs 5.1ms(release)。若用户 `cargo run`（dev）跑产品，一切成本 ×3.6。`just run`/产品 bin 的 profile 需确认；release 应为默认。

## F4 finalize reuse 在 dev 未命中（0/40）release 命中（38/40）

reuse fast path（body 相同跳 finalize）release 命中 38/40；dev 报告 0——疑 dev 下行内容含随机性（时间戳/计数）或测量帧选取差异，待 T2 实验复辨。非主要矛盾（finalize 本身 µs 级）。

## F5 流式节流已生效但全重排单价高

tick_paced（8 delta/paint）：paint 648us/次、CPU 66%（dev）；uncoalesced 543us×1786/s≈100%。节流架构对，**单价**（全重排）是唯一可优化杠杆——同 F2。

## F6 诊断基建欠账（已修，随本票入库）

`lab_ao_stream_delta_perf_report` 陈年坏：缺 `on_run_started`→idle fence 吞 TextDelta→断言挂（bdef3600 与 HEAD 同坏，非 c2891 回归）。worktree 已验一行修复。

## 结论与实验设计（T2）

1. **E1 stale 粒度**：spinner tick 只标 dock/anim 组件 stale，实测 busy 期 wheel p95（预期 <2ms release）——**主修复候选**
2. **E2 运行 profile**：确认产品 run=release；dev 另行提示
3. **E3 场景问卷**：用户卡顿复现条件（busy 流式中滚？idle 滚？dev/release？终端？）——**待用户**
4. F4 复辨并入 E1 实验观察
