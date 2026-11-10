---
depends_on: []
title: "Fix tool plugin resource handle states"
---

# 工具插件资源句柄状态修复

## Why

工具插件的进程 / 索引 / 结果状态问题（静态扫描，实施前复核）：

- **codemode epoch 全局取消**（C1，高）：Wasmtime Engine 的 epoch 是 Engine 级全局量且 CodemodeRunner 持久持有（tool.rs:14 OnceLock）——一个并发 run_code 超时 increment_epoch 会同时中断其它仍在跑的 run_code；无「已中断」状态可读，结果不可归因。
- **LSP/DAP 僵尸句柄**（C16，中）：Mutex<Option<AgentHandle>> 兼作阶段标记，后端崩溃后仍 Some 并向死进程发请求；ensure 又静默重启掩盖状态。
- **fff 索引 partial 位不可逆**（C18，中）：扫描超时置 partial 后永不置回；取消只对 glob 分支生效，fuzzy 分支不检查 abort；超时后 spawn_blocking 任务未回收。
- **bash 结果三 bool**（C20，低）：cancelled/timed_out/exit_code=None 三载体重叠，下游无法区分「被 kill」与「无退出码」。

## What Changes

- codemode：每 run 隔离取消（独立 Store/Engine 配置或 per-run epoch deadline 组），中断结果可归因到具体 run。
- LSP/DAP：句柄状态显式（NotStarted/Running/Dead），崩溃后请求显式报不可用 + 可选重启策略；与 c3250 的 ExternalClient 僵尸态修法对齐。
- fff：partial 位带「后台仍在扫」信号与 ready 事件；abort 全分支生效。
- bash：结果状态显式枚举（Completed/Killed/Timeout），exit_code 语义单一。

## Capabilities

- agent-codemode；infra-lsp / infra-dap；infra-fff-search；infra-bash。

## Impact

- 范围：plugins/xylitol-official-plugin-{codemode,lsp,dap,tools-fff,tools}。
- 验证：并发 run_code 独立取消单测；LSP kill 后工具报错 BDD；fff 取消即时性测试。

## Open Questions

1. codemode per-run 隔离选型：多 Store 共享 Engine + per-run deadline（推荐，wasmtime 支持 epoch deadline per store）还是每次新建 Engine？
2. LSP/DAP 崩溃自动重启是否默认开启？
