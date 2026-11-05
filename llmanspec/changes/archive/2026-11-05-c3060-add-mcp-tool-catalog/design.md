# Design — MCP tool catalog 与 profile 过滤

## 1. Catalog 归属

- catalog 按 connection key（c3055）建立：每个 pool entry 保存 `Vec<McpToolDescriptor>`。
- `McpToolDescriptor { server, name, description, schema }`。
- 本 change 只做 tools；prompts / resources 归 c3070。
- hub 暴露只读 `catalog()` 与 `catalog_for_session(session_id)`；面板字段归 c3075。

## 2. Profile 过滤

- MCP 工具公开名保持 `mcp__server__tool`。
- 工具集由 `resolve_tool_selection` 统一处理：profile / config `tools` 的 `+name` / `-name` 可增删 MCP 工具；未列名默认可见。
- 本 change 不新增第二套过滤语义；只保证 MCP 工具进入同一选择管线并有测试覆盖。
- 不同 config fingerprint 的同名 server 独立连接与 catalog；工具名冲突时按 key 隔离，不静默覆盖。

## 3. Tool list changed

- rmcp `ClientHandler::on_tool_list_changed` 置 dirty 标记（按 manager）。
- 下一轮 prepare / ensure 发现 dirty 时重新 `list_all_tools`，重建 adapter 与 catalog。
- 下一回合边界失效并重建 agent tool set（沿用首回合冻结 / 重定稿语义）；不得在当前 provider 可见 `tools[]` 热扩。
- 由单测覆盖 dirty 标记与刷新；不扩 BDD step。

## 4. Resume reconcile

- resume 从 graph head / build_context 读取上下文（c3206 已落地），按当前 profile 重建 agent tool set。
- MCP claims：resume 时按 session workspace set 重新 claim；未使用 server 释放（c3055 pool）。
- 不冻结旧 profile；profile 变更由 c3100 处理。

## 5. 失败与诊断

- list tools 失败：保留上一份 catalog 并产生诊断；不得把失败当空 catalog。
- 同名工具冲突：按 connection key 保留；全局注册表只保留一份可见名，冲突记诊断。
