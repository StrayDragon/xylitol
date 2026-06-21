---
depends_on: []
---

# c86-timing-instrumentation: 启动计时仪表化

## Why
pi 的 `timings.ts`（31 LOC）提供 `PI_TIMING=1` 环境变量驱动的启动性能分析：
- `resetTimings()` / `time(label)` / `printTimings()`
- 记录各步骤耗时（ms）并在启动完成时输出总览

xylitol 完全缺失计时能力，无法快速定位启动瓶颈（如 ResourceLoader、ModelRegistry 加载耗时）。

## What Changes
- 新增 `src/infra/timing.rs`：
  - 条件启用 `XYLITOL_TIMING=1`（环境变量）
  - `TimingCollector` — 全局计时器
  - `reset_timings()` / `time(label: &str)` / `print_timings()`
  - 使用 `std::time::Instant` 测量
  - 输出到 stderr（不污染 stdout）
- 在 CLI startup 关键路径插入计时点：
  - config 加载
  - ResourceLoader.reload()
  - ModelRegistry 加载
  - SessionManager 恢复
  - AgentSession 创建

## Capabilities
- diagnostics

## Impact
- 非破坏性：纯新增模块 + 有条件执行的 timing 调用。
- 无新依赖。环境变量未设置时零开销（空函数 inline）。
