# Design — 工具插件资源句柄状态（2026-11-13 复核后定稿）

## 1. 目标与边界

- codemode 并发提交取消互不干扰且可归因；LSP/DAP 句柄崩溃显式化；fff 取消全分支生效；bash 结果显式枚举。
- 边界：不改 Wasmtime 隔离模型；不改 LSP/DAP 配置格式。

## 2. 决策摘要（2026-11-13 复核后全部定稿）

| 决策 | 内容 | 状态 |
|---|---|---|
| codemode 隔离 | 多 Store 共享 Engine；每 store 配 epoch_deadline_callback（闭包读本 run 的 Arc<AtomicBool> interrupted：已置位 → Err(trap)，否则 → Ok(UpdateDeadline::Continue(1)) 续跑）；超时/取消置本 run 标志后 increment_epoch | 定稿（探针：wasmtime 50.0.1 有 Store::epoch_deadline_callback 返回 UpdateDeadline，无 per-store notify；callback + Continue(1) 即提交级隔离） |
| 中断归因 | CodemodeErrorKind::Timeout / Cancelled 已区分 reason；错误消息带 ctx.call_id 归因到具体提交 | 定稿（修正：不新增 Interrupted 结构体，错误类型已有 reason，call_id 进消息） |
| LSP/DAP 相位 | AgentPhase { NotStarted, Running, Dead(reason) } 替换 Mutex<Option<AgentHandle>>；请求传输错误 → Dead(reason) 并返回 unavailable 错误 | 定稿 |
| 自动重启 | 不默认；Dead 后下一次调用重建一次并 log::warn 记 restart 观测；重建失败返回 unavailable | 定稿 |
| fff abort | 检查提升到 fuzzy / glob / regex 共用结果迭代层；partial 输出附「后台仍在扫」信号 | 定稿 |
| bash 结果 | XyBashResult 加 outcome: Completed / Killed / TimedOut；exit_code 语义单一（仅真实退出码） | 定稿 |

## 3. codemode 隔离结构

Engine 单例（tool.rs OnceLock）；每 run 独立 Store + set_epoch_deadline(1) + epoch_deadline_callback；超时/取消任务置本 run 的 interrupted 标志并 increment_epoch；其它 run 的 callback 返回 Continue(1) 不受影响。

## 4. 测试边界（seam）

- 单测：并发两个 run_code，一个超时不影响另一个完成；LSP/DAP Dead 后请求报 unavailable、下一次重建；fff 取消即时；bash outcome 枚举。
- BDD：infra-lsp / infra-dap kill 后工具报错；infra-fff 取消即时性。

## 5. 风险与回滚

- epoch callback 语义依赖 wasmtime 版本（=50.0.1 锁定）；codemode / lsp-dap / fff / bash 四片独立回滚。

## 7. 实施前复核（2026-11-13）

| 结论 | 复核 | 证据 |
|---|---|---|
| codemode epoch 全局 increment 打断并发 run | 成立 | runner.rs:187 set_epoch_deadline(1)；:230/:236 engine.increment_epoch()；Engine 为 CodemodeRunner 单例（tool.rs:14 OnceLock） |
| wasmtime 50 无 per-store notify、有 epoch_deadline_callback | 成立 | ~/.cargo/registry/.../wasmtime-50.0.1/src/runtime/store.rs:1147-1151 |
| LSP/DAP Mutex<Option<AgentHandle>> 兼作相位 | 成立 | lsp/lib.rs:95-130；dap/lib.rs:105-140 |
| fff partial 位不可逆、fuzzy 分支不查 abort | 成立 | index.rs:11,28-31；find.rs:124-150,190-245 |
| bash 三 bool 重叠 | 成立 | sdk ports/bash.rs:10-17 |
