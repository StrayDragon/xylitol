# TUI 渲染性能与内存基线报告（c2490）

> 首次系统化基线。三个 lab 探针（`#[ignore]`，人工跑、不入 qa 闸）：
> `lab_ao_session_perf_report` / `lab_ao_stream_delta_perf_report` / `lab_ao_memory_profile`，
> 全部位于 `src/app/tui/lab_ao_perf.rs`。

## 机器与口径（2026-09-01）

- CPU：AMD Ryzen AI 9 H 365（20 线程）；RAM 30Gi；Linux 7.2.2-arch1；rustc 1.97.1。
- 合成终端 120×40；内容为 CJK 文本（每行 48 字，assistant 块 8 行/条目）。
- 数据两列：**dev**（`cargo test --lib`，debug 断言/无优化）与 **release**（更有代表性）。
- 内存口径：`/proc/self/status` 的 `VmRSS`，同进程内 1k→1w→10w 行递增（模拟会话原地增长），
  每档 warm 渲染 + settle 后采样；首档 delta 含一次性基础设施分配，末档边际值最可信。
- 帧行数（component_lines）为渲染器实测，非文本行估算。

## 结论速览（release）

| 指标 | 数值 | 含义 |
|---|---|---|
| 稳态交互重绘（wheel/idle） | p50 5–25us，max ≤29us，40/40 帧纯 reproject | **交互成本 O(视口)，AO 架构达标，无需优化** |
| 拖拽选择重绘 | p50 93us / max 199us | 仍在亚毫秒级 |
| 流式合帧收益 | tick-paced 比 per-token 少 8× 绘制、省 7× 渲染时间 | 合帧策略有效，维持 |
| 恢复投影（warm render） | 1.5k 行 28ms → 15k 367ms → 150k **9.96s** | **O(行数) 线性恶化——冷恢复是真实痛点** |
| 内存边际成本 | **≈1.42MB / 千行组件行**（1w 与 10w 档收敛一致） | 10w 行 ≈ 200MB 常驻 |
| paint_lines | 所有规模恒为 40（=终端行数） | AO 视口投影契约在 15 万行下不破 |

## 帧成本（lab_ao_session_perf_report，合成 ~80 turns / 561 组件行）

| 阶段 | dev p50/p95/max | release p50/p95/max |
|---|---|---|
| warm（首次全投影） | — | 11.1ms |
| idle 强制重绘 | 18/24/…us | 5/8/10us |
| wheel 滚动 | 82/95/…us | 24/25/29us（40/40 reproject-only） |
| 拖拽选择 | 3xx/…us | 93/140/199us |
| 末帧 do_render | — | 195us，reprojected=true |

（完整原始 REPORT 行见文末附录；dev 列同口径。）

## 流式绘制（lab_ao_stream_delta_perf，160 deltas / tick_every=8，release）

| 策略 | paints | paints/s | Σdo_render | CPU% |
|---|---|---|---|---|
| per-token 强绘 | 160 | 2071 | 74.4ms | 207% |
| tick-paced（产品路径） | 20 | 1552 | 10.6ms | 155% |
| 收益 | **8.00×** 更少绘制 | | **7.04×** 更省渲染 | |

## 内存画像（lab_ao_memory_profile）

| 档位 | 组件行 | RSS（dev） | RSS（release） | 边际 KB/千行（release） |
|---|---|---|---|---|
| 基线 | 0 | 13.6MB | 10.3MB | — |
| 1k | 1,511 | 28.7MB | 17.8MB | 3,542（含一次性开销） |
| 1w | 15,011 | 47.1MB | 36.4MB | 1,383 |
| 10w | 150,011 | 239.8MB | 228.1MB | **1,420** |

- 边际成本在 1w→10w 间稳定（≈1.4MB/千行），说明增长是**线性无上界**：长会话没有缓存上限。
- 恢复投影时间同为线性：10w 行冷恢复 ≈10s（release）——用户可感知的卡顿点。

## 对后续优化票的含义（凭数据立项）

1. **优先级最高：冷恢复/投影路径的增量化**。warm render O(n) 且 10s@10w 行；
   steady-state 交互已经 O(视口)，瓶颈只在「一次性全量投影」。
2. **内存上限票次之**：线性 1.4MB/千行意味着 10w 行 ≈200MB；候选方向是缓存上限 +
   视口外丢弃（配合增量化投影一并设计）。
3. **交互重绘与流式合帧不需要动**：数据明确否决了「重绘已经太慢」的猜测方向。

## 局限

- 内容为合成 CJK 文本；真实会话（markdown/代码高亮/工具段）的每行成本可能更高，
  探针支持 `XYLITOL_LAB_SESSION=<uuid>` 覆盖真实会话复测。
- 同进程递增测内存含分配器滞留噪声（首档偏高，末档边际值可信）。
- RSS 采样为 Linux-only（`/proc`），与 `cpu_jiffies_self` 同口径。
- dev 数字仅供回归对照，优化判断以 release 列为准。

## 附录：原始 REPORT 行（release）

```text
REPORT lab_ao_memory_profile baseline_rss_kb=10288 cols=120 rows=40 block_lines=8
REPORT lab_ao_memory_profile scale=1k store_fill_ms=30 entries=251 component_lines=1511 paint_lines=40 warm_render_ms=28 rss_kb=17772 delta_rss_kb=5352 kb_per_1k_lines=3542.0
REPORT lab_ao_memory_profile scale=1w store_fill_ms=382 entries=2501 component_lines=15011 paint_lines=40 warm_render_ms=367 rss_kb=36448 delta_rss_kb=18676 kb_per_1k_lines=1383.4
REPORT lab_ao_memory_profile scale=10w store_fill_ms=10124 entries=25001 component_lines=150011 paint_lines=40 warm_render_ms=9963 rss_kb=228100 delta_rss_kb=191652 kb_per_1k_lines=1419.6
REPORT lab_ao_session_perf
source=synthetic:ao-perf-scroll
warm_us=11124 component_lines=561 paint_lines=40 finalize_checks=40 reuses=0
idle_paint p50=5us p95=8us max=10us
wheel_paint p50=24us p95=25us max=29us reprojected=40/40 vertical_shifted=40/40 ao_reproject_frames=64
drag_paint p50=93us p95=140us max=199us
last_frame component_lines=561 paint_lines=40 finalize_checks=2 reuses=38 do_render_us=195 ao_reprojected=true
REPORT lab_ao_stream_delta_perf
deltas=160 tick_every=8 max_component_lines=573
per_token_paint paints=160 paints_per_s=2071 sum_do_render_us=74367 wall_s=0.077 cpu%≈207.1
tick_paced      paints=20 paints_per_s=1552 sum_do_render_us=10565 wall_s=0.013 cpu%≈155.2
ratio paints=8.00x  sum_us=7.04x (higher ⇒ tick-pace saves more)
```

dev 对照（同口径）：baseline 13.6MB；1k→28.7MB/4725KB每千行；1w→47.1MB/1364.7；10w→239.8MB/1427.1；
warm_render_ms 79/745/18232；store_fill_ms 82/776/18443。
