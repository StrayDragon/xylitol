# Tasks — c1135-add-app-tui-startup-header

## 1. Driver 缝

- [x] 1.1 `LoadedResourcesSnapshot` / `McpHeaderSnapshot` + `Driver` 只读 API；InProcess/Scripted/Remote stub
- [x] 1.2 InProcess：skills 自 `loaded_skill_names`；MCP 自 `McpSession::connected_servers` + diagnostics（无密钥）

## 2. UI

- [x] 2.1 `UiRoot` loaded_resources 槽 + render（scrollback 前，至多 2 行）
- [x] 2.2 host `refresh_loaded_resources`；启动 + `/reload` 接线
- [x] 2.3 `DESIGN.md` + `design/loaded-resources.md`

## 3. 测与校验

- [x] 3.1 harness：skills 可见；MCP 可见；皆空 0 行；reload 刷新
- [x] 3.2 `llman sdd validate c1135… --strict --no-interactive`
- [x] 3.3 `just lint` + 相关 harness 测
