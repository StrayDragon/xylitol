# Design — Serve 网关相位编码收敛

## 1. 目标与边界

- 相位单一编码；对外状态字符串单点产出；死变体删除（needs_specs_change: false，对外值不变）。
- 边界：healthz / unavailable 文案与值不变。

## 2. 决策摘要

| 项 | 方向 |
|---|---|
| Gateway 相位 | Phase 增 Stopping 变体；AtomicU8 承载改为 Phase 的原子序号封装（单一定义，A8） |
| 状态字符串 | status_str() 单点，render_unavailable 与 healthz 共用（A8） |
| 生命周期 | lifecycle u8 常量改 ServeLifecycle 原子承载合一；未知值显式错误（A9） |
| ReverseRpcResult | Timeout 死变体删除（A18） |
| 监听器收尾 | shutdown 与 Drop 重复路径合一（A19） |

## 3. 测试边界

单测：相位转移与字符串产出；server-core BDD 回归（healthz 值不变断言）。
