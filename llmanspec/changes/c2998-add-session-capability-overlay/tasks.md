# Tasks

## 1. SDK 能力 DTO

- [x] 1.1 `xylitol-plugin-sdk`：新增 `session/capability.rs`，定义 `CapabilityKind` / `CapabilityEntry` / `CapabilitySnapshot`（serde snake_case）。
- [x] 1.2 `session/mod.rs` 再导出三个类型。

## 2. 运行时覆盖层

- [x] 2.1 `agent/tools/capability.rs`：实现 `tool_capability_id` / `mcp_capability_id` / `filter_tools_by_capability_overlay` / `capability_snapshot`。
- [x] 2.2 `XyDriver` 增加默认 `capability_snapshot` / `capability_pending` / `set_capability` / `clear_capabilities`。
- [x] 2.3 `XyInProcessDriver` 持有 `base_tools` / `capability_disabled` / `capability_applied` / `mcp_server_ids`，实现四个能力方法。
- [x] 2.4 `run` 开头 `apply_pending_capability_overlay()`，随后 `ensure_tool_table_frozen()`。
- [x] 2.5 `apply_prepare_tools` 更新 base 后按覆盖层过滤；`McpReload` 改走 `install_base_tools`。
- [x] 2.6 内核装配路径 `set_base_tools` + `set_mcp_server_ids`（bootstrap / occupancy factory / composition）。
- [x] 2.7 单测：工具与 MCP 过滤、pending 标记、未连接服务器入快照。

## 3. 协议命令

- [x] 3.1 `Command` 新增 `GetCapabilities` / `SetCapability` / `ClearCapabilities`。
- [x] 3.2 `REGISTRY` 登记三行（Readonly/Writer，Inline）并更新守卫测试期望。
- [x] 3.3 内核 `DispatchOutcome::Capabilities` + 命令分派 + JSON 序列化。
- [x] 3.4 协议测试：注册行、Exec 列、command_backed 穷举。

## 4. TUI 覆盖盘

- [x] 4.1 `layout/slots/capabilities.rs` 接入 `slots/mod.rs`（模块 + `EditorSlot::Capabilities` + 输入/渲染/主题/足迹）。
- [x] 4.2 `UiRoot` 增加 `mount_capabilities` / `take_pending_capability_toggle` / `close_capabilities` 等辅助。
- [x] 4.3 `PendingSlash::OpenCapabilities` + `parse_slash_command` + BusySlashPolicy + SlashCommandSource。
- [x] 4.4 `effects/slash/misc::open_capabilities` 与 `effects/pending_ui/capabilities.rs`：拉取快照挂槽、toggle 后提交 `SetCapability` 并刷新。
- [x] 4.5 `sync_fixed_zone` 在 `driver.capability_pending()` 时显示 `Next turn: capability overlay`。
- [x] 4.6 Esc 关槽使用既有 `app.interrupt` 绑定，不引入不存在的 keybinding。
- [x] 4.7 `ScriptedDriver` 实现能力方法并记录调用，供 TUI BDD 使用。

## 5. 规格与 BDD

- [x] 5.1 `agent-session`：新增会话能力覆盖层规则（会话内临时 / 下一回合生效 / 结束即丢）。
- [x] 5.2 `protocol-app`：新增能力方法表登记规则。
- [x] 5.3 `app-tui-commands`：新增 `/capabilities` 覆盖盘与 pending 提示规则。
- [x] 5.4 补 step 定义与 `#[scenario]` 绑定，`cargo test --all-features --test bdd` 全绿。

## 6. 门禁与收尾

- [x] 6.1 `just qa` 全绿（含 clippy / 注释闸 / 版本命名闸）。
- [x] 6.2 `just qa-e2e` 全绿。
- [x] 6.3 `llman-sdd validate c2998-add-session-capability-overlay --check --strict` 通过。
- [x] 6.4 verify 报告无 CRITICAL，finalize 归档。
