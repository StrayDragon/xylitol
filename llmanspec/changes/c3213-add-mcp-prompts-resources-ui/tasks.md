# Tasks — MCP prompts and resources UI

- [ ] 1. TUI：扩展 `/mcp` 子命令 `prompt`；Choice 列出当前 scope prompt；`prompts/get` 结果插入 editor，不自动发送。
- [ ] 2. TUI：`@` 资源选择器；按 server / uri / name 搜索；插入引用 chip；发送时解析。
- [ ] 3. Host：`mcp.prompt.get` / `mcp.resource.read` unary；读取失败可操作错误；发送前失败阻止提交。
- [ ] 4. 订阅：连接后 `Peer::listen`；filter 含 `resources_list_changed` 与已选 URI；`Subscription::next` 更新 catalog / 缓存。
- [ ] 5. 缓存：connection key + URI；LRU 64；单条 256 KiB；TTL 5 分钟；更新/列表变更失效。
- [ ] 6. Specs：`infra-mcp` prompts/resources UI 与订阅规则；单测 / BDD。
- [ ] 7. 门禁：`just fmt` / `just lint` / `just qa` / `just qa-e2e`。
