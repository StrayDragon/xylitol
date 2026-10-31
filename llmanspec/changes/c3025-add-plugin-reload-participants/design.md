# 设计：插件具名重载参与者

## SDK

- `ReloadEvent { cwd, agent_dir, project_trusted }`。
- `ReloadParticipant`：`name() -> &'static str`；`reload(&ReloadEvent) -> Result<String, String>`。
- `ReloadHub`：注册与快照（顺序保持）。

## 宿主

- `registries` 核心插件 `ctx.provide(ReloadHub::new())`。
- 运行器：宿主内置参与者 → `driver.ctx()` 的 `ReloadHub` 参与者；事件由 cwd / agent_dir / trust 构造；取消与失败语义同宿主参与者。

## mcp 插件

- `install_prepare_hub` 注册 `McpReloadParticipant`（`name = "mcp"`，消息 `session/prepare reload fired`）。
- 真实 MCP 刷新仍经 `SessionPrepareReason::Reload` 事件；该参与者只报告步骤。

## 规格与架构

- 规格无需改（r1191 已为开放集合）。
- `src/AGENTS.md` 增补「重载参与者」：命令只遍历注册表、不得内联清单；内置经 kernel 装配、插件经 `ReloadHub`、端侧本地机制另属客户端。
