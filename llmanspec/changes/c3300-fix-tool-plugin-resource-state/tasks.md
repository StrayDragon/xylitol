# Tasks — 工具插件资源句柄状态修复

> 实施者导读：design §7 是复核结论；wasmtime 探针结论见 §2。收口走 finalize，不列为任务。

- [ ] T1 spec delta：agent-codemode r3347；infra-lsp r3348 / infra-dap r3349；infra-fff-search r3350；infra-bash r3351
- [ ] T2 codemode：per-store epoch_deadline_callback + interrupted 标志 + call_id 归因；并发隔离单测
- [ ] T3 LSP/DAP：AgentPhase 枚举替换 Option；Dead 显式报 unavailable；下次调用重建 + 观测；单测
- [ ] T4 fff：fuzzy 分支 abort 检查；partial 输出附后台扫描信号；单测
- [ ] T5 bash：XyBashResult.outcome 枚举；调用方与测试更新
- [ ] T6 回归：just fmt / lint / test / qa；llman-sdd validate c3300 --strict 绿
