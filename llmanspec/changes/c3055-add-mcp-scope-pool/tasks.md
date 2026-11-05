# Tasks — MCP scope pool 与连接生命周期

- [ ] 1. Scope / credential / connection key：config 字段、fingerprint、key 构造与隔离单测。
- [ ] 2. Pool + 引用计数 + 懒连接：claim / release、同 key 共享、最后释放 shutdown；单测。
- [ ] 3. 生命周期：Switch/Resume 不 reset、Reload 清空、连接失败按 server 重连；单测。
- [ ] 4. 快照与 view：保留上次已知快照，忙时不把未知当未配置；单测。
- [ ] 5. SessionPrepare workspaces：事件字段、runtime 填充、hub 按 workspace 建 claim；stdio current_dir；单测 / fixture。
- [ ] 6. Specs 与测试：infra-mcp / agent-runtime 条款；BDD 多 workspace 断言。
- [ ] 7. 门禁：`just fmt` / `just lint` / 相关测试 / `llman-sdd validate c3055-add-mcp-scope-pool --strict` / `just qa`。
