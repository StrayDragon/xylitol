# language: zh-CN
# managed by llman sdd partition-migrate
功能: infra-diagnostics

  @req:d0
  场景: placeholder
    假如 未启用诊断
    当 无操作
    那么 无变化

  @req:d1
  场景: enabled
    假如 XYLITOL_TIMING 设为 1
    当 启动运行
    那么 收集并打印 timing 数据

  @req:d1
  场景: disabled
    假如 XYLITOL_TIMING 未设置或为 0
    当 启动运行
    那么 无 timing 输出

  @req:d2
  场景: points
    假如 启动经所有阶段
    当 每阶段调用 time
    那么 5 个 timing points 均被记录

  @req:d3
  场景: stderr-output
    假如 timing 数据已收集
    当 print_timings 被调用
    那么 输出到 stderr，含每步 ms 与 total
