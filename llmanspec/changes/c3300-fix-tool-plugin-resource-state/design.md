# Design — 工具插件资源句柄状态

## 1. 目标与边界

- codemode 并发 run 取消互不干扰且可归因；LSP/DAP 句柄崩溃显式化；fff 取消全分支生效。
- 边界：不改 Wasmtime 隔离模型；不改 LSP/DAP 配置格式。

## 2. 决策摘要

| 决策 | 内容 | 状态 |
|---|---|---|
| codemode 隔离 | 多 Store 共享 Engine；per-run epoch deadline + 超时任务精确 notify 单个 store（wasmtime Store 级 API） | 推荐，待 review |
| 中断归因 | run 结果带 Interrupted { reason: Timeout / Cancelled, run_id } | 定稿方向 |
| LSP/DAP 相位 | AgentPhase { NotStarted, Running, Dead(reason) }；请求遇 Dead 显式报 unavailable | 定稿方向 |
| 自动重启 | 不默认；Dead 后下次调用重建并记 restart 观测 | 推荐，待 review |
| fff abort | 检查提升到 fuzzy/glob 共用迭代层；partial 附带后台扫描信号 | 定稿方向 |
| bash 结果 | XyBashResult.outcome: Completed / Killed / TimedOut，exit_code 语义单一 | 定稿方向 |

## 3. codemode 隔离结构

Engine 单例；每 run 独立 Store 配 set_epoch_deadline；超时任务 sleep 后仅 notify 该 run 的 store。实施首任务核对当前 wasmtime 版本的 Store::epoch_deadline_callback / notify_epoch_deadline 可用性（lab 探针）；不可用则退化方案：每次 run 独立 Engine（冷启动开销换隔离正确性）。

## 4. 测试边界（seam）

- 单测：并发两个 run_code，一个超时不影响另一个完成且归因正确。
- BDD：infra-lsp / infra-dap kill 后工具报错；infra-fff 取消即时性。

## 5. 风险与回滚

- wasmtime API 匹配度是首任务；codemode / lsp-dap / fff / bash 四片独立回滚。
