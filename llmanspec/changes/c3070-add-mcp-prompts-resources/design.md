# Design — MCP prompts and resources

## 1. Prompts

- 连接后 list_all_prompts；每个 prompt 生成模型工具 mcp__server__prompt__name。
- 参数 schema 由 prompt arguments 合成（object + properties + required）。
- 调用走 prompts/get；按 server call_timeout 超时。

## 2. Resources

- list_all_resources；每个 server 生成一个 mcp__server__resource__read 工具，参数 uri。
- resources/templates/list 的模板由同一 read 工具覆盖（传具体 uri）。
- 调用走 resources/read；按 server call_timeout 超时。

## 3. Catalog

- hub catalog 增加 kind（tool / prompt / resource）；prompt/resource 同样按 connection key 隔离。
- profile tools 的 +/- 选择同样作用于 prompt/resource 工具名。

## 4. 未做（本 change 边界）

- subscriptions/listen 未接；server 资源更新通知不进入 catalog 刷新。
- TUI 的 slash prompt 选择器与 @ 资源选择器未做；prompt/resource 仅以模型工具暴露，/mcp 面板显示条目（c3075）。
- 上述未做项不得在 tasks/specs 中写成已交付。

## 5. 测试

- prompt adapter 名称与 schema；resource adapter schema 与缺 uri 拒绝。
- 现有 fixture 仍只覆盖 tools；prompt/resource 的真实 server 断言留给 stateful fixture 后续。
