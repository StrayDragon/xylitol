# Design — c2490 TUI 渲染性能与占用基线

## 现状核对（代码事实）

- `src/app/tui/lab_ao_perf.rs` 已有两个 `#[ignore]` lab 探针：
  - `lab_ao_session_perf_report`：warm/idle/wheel/drag 帧耗时 p50/p95、AO reproject 复用率、paint_lines 闸；
  - `lab_ao_stream_delta_perf_report`：流式合帧对比（per-token 强绘 vs tick-paced），paints、Σ do_render_us、CPU%（`/proc/self/stat` jiffies）。
- 结论：**帧成本尺子已大半存在**；本票真实缺口是「内存占用画像」与「结果落档口径」。

## 决策

- **D1 落点：扩展 `src/app/tui/lab_ao_perf.rs`，不新建 example/binary。**
  内存画像需要进程内访问 TUI 状态（scrollback/fold 缓存），lib 内 `#[ignore]` test 位置最合适；
  LabTerminal / percentile / `/proc` 采样惯例全部复用。
- **D2 内存口径：`/proc/self/statm` RSS 采样。**
  与既有 `cpu_jiffies_self` 同源（Linux-only 先例成立）；跨平台抽象等有真实需求再说（pre-0.0.1 不预留）。
- **D3 规模档：1k / 1w / 10w 行级 transcript，默认合成注入**（`debug_fixtures::seed_scene` 或循环注入
  TextDelta / 工具段），`XYLITOL_LAB_SESSION` 覆盖真实磁盘会话的既有惯例保留。
  固定态测常驻：warm 渲染完成后静置采样，不含流式峰值。
- **D4 落档口径：`REPORT` 行统一 key=value**（沿用现有格式），人工跑、结果粘贴进
  `research/perf-baseline-report.md`（apply 阶段产出）。不进 `just qa`（`#[ignore]` 惯例）。
- **D5 `skip_specs_landing: true`**：纯 lab 测量，无可观察产品行为合约。

## 非目标（沿 proposal）

不做优化实现；不建 benchmark CI；不为测量改变生产代码路径行为。
