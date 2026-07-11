# Tasks — c535-refactor-server-onto-driver

- [ ] 1. 盘点 server REST/WS 对 agent 的直接调用点
- [ ] 2. AppState 改为持有 InProcessDriver（+ McpSession）
- [ ] 3. run / abort / 模型与会话命令改经 Driver 或 dispatch
- [ ] 4. 删除 into_agent 过渡路径；更新 server 测试 / BDD
- [ ] 5. `llman sdd validate c535-refactor-server-onto-driver --strict --no-interactive`
- [ ] 6. `just lint` + server 相关测试
