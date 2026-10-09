# language: zh-CN
# capability: infra-diagnostics
# purpose: 诊断 — 启动计时 instrumentation 与性能 profiling。
# scope: src/infra/timing.rs

功能: infra-diagnostics

  @req:r1437
  规则: timing-collector
    System MUST 提供 reset_timings、time label 与 print_timings，由 XYLITOL_TIMING 环境变量门控。
    # verified-by: src/infra/timing.rs

    场景: timing-collector-gated
      当 读取计时收集器边界
      那么 收集器由 XYLITOL_TIMING 门控且含重置与计时
  @req:r1438
  规则: timing-points
    System MUST 在 config load、ResourceLoader reload、ModelRegistry load、session restore 与 session create 插入 timing points。
    # verified-by: src/infra/timing.rs

    场景: timing-points-on-critical-path
      当 读取计时调用点清单
      那么 启动关键路径含计时点
  @req:r1439
  规则: timing-output
    print_timings MUST 向 stderr 输出每步 ms 与 total。
    # verified-by: src/infra/timing.rs

    场景: timing-output-millis
      当 读取计时输出格式
      那么 每步 ms 与合计可观测
