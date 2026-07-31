# Tasks: c1200-update-infra-mcp-startup

## 1. Specs + 绑定

- [x] 1.1 收紧 live `infra-mcp`：非阻塞启动 / 并行连接 / 进度可观测（新 req）
- [x] 1.2 收紧 live `app-tui-host` / `app-tui-chrome`：loaded-resources mcp 进度；新会话与 CLI resume 不挡；面内 resume 不重连
- [x] 1.3 对应 `*.feature` + `llman sdd validate … --strict --no-check`
- [x] 1.4 `llman sdd change start c1200-update-infra-mcp-startup` → branch `sdd/c1200-…`（Stage: full）

## 2. 连接与 Driver 缝

- [ ] 2.1 并行 `connect_servers` + 进度/快照字段
- [ ] 2.2 CLI：进 TUI/print 前不再阻塞完整 MCP bootstrap；后台任务 + ready 热合并 tools
- [ ] 2.3 Driver 只读缝供 host 刷新 loaded-resources（含 connecting）

## 3. 产品面

- [ ] 3.1 loaded-resources mcp 行渲染 connecting / ready / 失败摘要
- [ ] 3.2 CLI `--session`：rebuild 与 MCP 并行（验收：未 ready 也可见历史）
- [ ] 3.3 面内 `/session-resume`：不触发阻塞重连
- [ ] 3.4 connecting 期间闸 agent prompt（及 bang）；允许 slash/滚历史；拒绝短提示；结算后解闸

## 4. 验证

- [ ] 4.1 无配置 zero-cost 不回归
- [ ] 4.2 相关 `cargo test` / harness + validate --strict
- [ ] 4.3 `/reload` 既有用例不回归（不做 c1205）
