# language: zh-CN
# purpose: 诊断 — 启动计时 instrumentation 与性能 profiling。

功能: infra-diagnostics

  @req:r1437
  规则: timing-collector
    System 须提供 reset_timings、time label 与 print_timings，由 XYLITOL_TIMING 环境变量门控。

    场景: 计时收集器门控
      当 读取计时收集器边界
      那么 收集器由 XYLITOL_TIMING 门控且含重置与计时
  @req:r1438
  规则: timing-points
    System 须在 config load、ResourceLoader reload、ModelRegistry load、session restore 与 session create 插入 timing points。

    场景: 计时指向时关键路径
      当 读取计时调用点清单
      那么 启动关键路径含计时点
  @req:r1439
  规则: timing-output
    print_timings 须向 stderr 输出每步 ms 与 total。

    场景: 计时输出毫秒
      当 读取计时输出格式
      那么 每步 ms 与合计可观测
