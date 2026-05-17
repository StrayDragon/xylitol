# c85-add-dap-layer Tasks

- [ ] 定义 DapClient trait（attach/set_breakpoints/continue/get_variables/get_stack_trace/disconnect）
- [ ] 定义 DAP 相关数据结构（StackFrame, Variable, DapEvent 等）
- [ ] 添加 YAML 配置入口（dap.enabled, dap.backends）
- [ ] 实现 feature flag 门控（feature = "dap"）
- [ ] 编写 trait 编译测试（无外部 dap 后端依赖）
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c85-add-dap-layer --strict --no-interactive`
