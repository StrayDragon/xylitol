# Tasks: c1545-add-tool-batch-execution-mode

> Apply 闸已开（Q1–Q8 关闭；MCP 硬串行）。垂直切片。

## 1. 合约（propose）

- [x] 1.1 live `agent-runtime`：ar27–ar29 + feature
- [x] 1.2 live `agent-tools`：t26–t27（MCP 硬 Barrier）
- [x] 1.3 live `runtime-config`：rc26（仅 mode）
- [x] 1.4 live `infra-mcp`：mcp6
- [x] 1.5 live `infra-otel`：otel18
- [x] 1.6 `llman sdd change attach`
- [x] 1.7 关闭 Open Questions；MCP 无 glob/配置放行；强化 design harness/架构

## 2. 类型与分类 SSOT

- [x] 2.1 引入 `BatchMode` + `ToolConcurrency`（迁移/别名旧 `XyToolExecutionMode`）
- [x] 2.2 内置：write/bash/edit → Barrier；读族 ParallelSafe；单测表
- [x] 2.3 `McpToolAdapter` 固定 Barrier；`classify` 对 `mcp:` **硬短路**（即使 trait 撒谎）
- [x] 2.4 单测：无配置字段可放行 mcp；`tool_batch` 仅 mode

## 3. 调度器（无 I/O）

- [x] 3.1 `agent/runtime/tool_batch.rs`：classify + plan_windows
- [x] 3.2 单测：读/写/读切窗；全 safe；全 barrier；`mcp:` 夹在中间强制 BarrierOne

## 4. 执行层 + ReAct

- [x] 4.1 抽出 `tool_exec.rs`（preflight / Update mux / after / persist）
- [x] 4.2 Sequential 路径兼容（既有 BDD 绿）
- [x] 4.3 BarrierParallel：flush 窗 + cancel
- [x] 4.4 history 源序；End 可完成序
- [x] 4.5 Session/配置注入；setter 下轮生效

## 5. 配置

- [x] 5.1 `AppConfig.tool_batch.mode` Default + 非法失败；**无** patterns 字段
- [x] 5.2 组合根 → Session

## 6. 观测

- [x] 6.1 并行显式 parent Span
- [x] 6.2 属性 `tool_batch.mode` / `barrier_index`
- [x] 6.3 [SHOULD] 同 iteration 子树证明

## 7. Harness

- [x] 7.1 测试假工具 `slow_safe` / `slow_barrier`
- [x] 7.2 BDD steps：`batch-default-sequential` / overlap / windows / history-order
- [x] 7.3 单测 S2–S5（含 mcp 硬 Barrier）
- [x] 7.4 `llman sdd validate c1545… --no-check`；窄 `cargo test` + 相关 BDD

## 8. 收尾

- [x] 8.1 删「整批塌缩」旧注释；去掉 `_tool_mode` discard
- [x] 8.2 fmt / lint；react 体量说明
