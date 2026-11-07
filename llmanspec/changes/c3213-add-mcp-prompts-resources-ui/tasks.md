# Tasks — MCP prompts and resources UI

- [ ] 1. TUI：扩展 `/mcp prompt`；prompt 选择器；有参数用 `ChoicePrompt` 自定义输入；结果插入 editor，不自动发送。
- [ ] 2. TUI：`/mcp resource` 与 `@mcp:` 补全；选中加入 editor 上方附件 tray；点击 `×` / Backspace（空 editor）/ 快捷键移除；同一 `(server, uri)` 去重；未加引号手写 `@mcp:` 按普通文本。
- [ ] 3. Host：`mcp.catalog`（prompts/resources/counts）与 `mcp.resource.read` unary；读取失败可操作错误。
- [ ] 4. Context seam：`XyDriver::run_with_context`；`ResourceContext`；协议 command 载荷 + wire 映射；runtime 写 `CustomMessage`；TUI 渲染可折叠 context 块。
- [ ] 5. 可选引号 token：`@mcp:"<server>/<uri>"` 解析；转义、代码块跳过、未知资源阻止发送、与 tray 去重。
- [ ] 6. 订阅与缓存：`Peer::listen`；`resources_list_changed` + 已附加 URI；LRU 64 / 256 KiB / TTL 5 分钟；更新失效。
- [ ] 7. Specs / 测试：`infra-mcp` prompts/resources UI、tray、context seam、引号 token 规则；单测 / BDD。
- [ ] 8. 门禁：`just fmt` / `just lint` / `just qa` / `just qa-e2e`。
