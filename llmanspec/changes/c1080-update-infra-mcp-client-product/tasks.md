# Tasks — c1080-update-infra-mcp-client-product

## 1. 配置与诊断

- [ ] 1.1 校验 stdio / url·sse 必填字段；错误信息可读
- [ ] 1.2 连接失败：warn + 可查询诊断；bootstrap 不整崩
- [ ] 1.3 单测：坏配置 / 失败连接路径

## 2. 只读快照

- [ ] 2.1 `connected_mcp_servers`（或等价）含 id / transport / tool_count
- [ ] 2.2 zero-cost：无配置 → 空快照且无 client
- [ ] 2.3 reload 后快照反映新集合（复用 mcp3）

## 3. 校验

- [ ] 3.1 `LLMANSPEC_BASE_REF=main llman sdd validate c1080-update-infra-mcp-client-product --no-interactive`
- [ ] 3.2 `just qa`（或 lint + 相关 MCP 测）
