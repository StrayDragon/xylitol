# Tasks — MCP tool catalog 与 profile 过滤

- [ ] 1. Catalog：按 connection key 保存 tool descriptor，hub 暴露只读 catalog；单测。
- [ ] 2. Profile 过滤：MCP 工具进入 `resolve_tool_selection`，`+/-mcp__server__tool` 生效；单测。
- [ ] 3. Tool list changed：ClientHandler dirty 标记 + 下一轮刷新 catalog / adapter；单测。
- [ ] 4. Resume reconcile：resume 重建 tool set 与 claims；单测。
- [ ] 5. Specs 与测试：infra-mcp / agent-runtime 条款；单测。
- [ ] 6. 门禁：`just fmt` / `just lint` / 相关测试 / `llman-sdd validate c3060-add-mcp-tool-catalog --strict` / `just qa`。
