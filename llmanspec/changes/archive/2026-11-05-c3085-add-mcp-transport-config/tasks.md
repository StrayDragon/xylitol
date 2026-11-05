# Tasks — MCP transport 配置

- [x] 1. Transport canonical：`stdio` / `http`；`sse` 破坏性拒绝与迁移提示；配置校验；单测。
- [x] 2. Per-server timeout：connect / call timeout 字段与缺省；非法值校验；单测。
- [x] 3. Legacy SSE 诊断：URL / 协议错误产生可读错误，不自动回退；单测。
- [x] 4. Specs 与测试：infra-mcp 条款；单测。
- [x] 5. 门禁：`just fmt` / `just lint` / 相关测试 / `llman-sdd validate c3085-add-mcp-transport-config --strict` / `just qa`。
