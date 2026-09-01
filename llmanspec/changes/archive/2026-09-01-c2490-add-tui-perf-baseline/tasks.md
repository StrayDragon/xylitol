# Tasks — c2490 TUI 渲染性能与占用基线

- [x] 1. `src/app/tui/lab_ao_perf.rs` 新增内存画像探针 `lab_ao_memory_profile`（`#[ignore]`）：
  1k / 1w / 10w 行合成 transcript → warm 渲染后常驻 RSS（`/proc/self/statm`）、
  component_lines vs paint_lines、fold/scrollback 缓存规模；`XYLITOL_LAB_SESSION` 可覆盖真实会话。
- [x] 2. 既有两探针输出补齐 `max` 档（现仅 p50/p95）并统一 `REPORT` key=value 口径，
  流式场景补「每秒绘制次数」（paints / wall_s，`lab_ao_stream_delta_perf_report` 已有原料）。
- [x] 3. 人工跑三探针，结果落档 `research/perf-baseline-report.md`（含命令行、机器口径说明）。
- [x] 4. 门禁：`just fmt` / `just lint` / `just test`（lab 为 `#[ignore]`，不入闸但需保证非 ignore 路径零回归）。
