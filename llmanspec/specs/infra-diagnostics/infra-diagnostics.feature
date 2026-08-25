# language: zh-CN
# capability: infra-diagnostics
# purpose: 诊断 — 启动计时 instrumentation 与性能 profiling。
# scope: infra 层 diagnostics

功能: infra-diagnostics

  @req:d1 @human
  场景: timing-collector
    - System MUST 提供 reset_timings、time label 与 print_timings，由 XYLITOL_TIMING 环境变量门控。

  @req:d2 @human
  场景: timing-points
    - System MUST 在 config load、ResourceLoader reload、ModelRegistry load、session restore 与 session create 插入 timing points。

  @req:d3 @human
  场景: timing-output
    - print_timings MUST 向 stderr 输出每步 ms 与 total。
