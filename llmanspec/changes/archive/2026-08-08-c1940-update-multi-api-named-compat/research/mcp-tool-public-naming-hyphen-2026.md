# MCP 公开工具名与连字符（2026-08）

> **问题**：server / tool 名含 `-`（如 `auto-mem`、`get-something`）时，主流 coding agent 如何拼「给模型看的」工具名？能否正确 dispatch？是否依赖反向解析？
>
> **证据**：MCP 官方 spec；Cursor 社区论坛（含员工确认）；本机 `../codex/codex-rs/`；Claude Code / Agent SDK 官方文档。未改 `llmanspec/specs`。

## 执行摘要

| 系统 | 模型侧形态 | `-` / `.` | Dispatch 是否靠反解析拼串？ |
|---|---|---|---|
| **Cursor** | `CallMcpTool({ server, toolName, … })` 分字段 | 早期 agent 对 `-` 过敏；2026-04 员工称 `-` 已支持；`.` 警告并常被换成 `_` | **否**（结构化字段） |
| **Codex** | namespace `mcp__{sanitized_server}` + name `{sanitized_tool}`；展示常拼成 `mcp__…__…` | sanitize：非 `[A-Za-z0-9_]` → `_`（`-`/`.` 都变 `_`） | **否**（保留 raw `server_name` + `tool.name`） |
| **Claude Code / Agent SDK** | `mcp__{server}__{tool}`；plugin 形 `mcp__plugin_{p}_{s}__{t}` | plugin 文档：保留 `_`/`-`；其它非法字符 → `_` | 文档强调前缀路由；**不**要求从 tool 名反推 MCP wire |
| **MCP spec** | 单 server 内 `name`；聚合客户端 **MAY** 加前缀消歧 | **允许** `A-Za-z0-9_-.`；1–128 字符（SHOULD） | N/A（wire 始终是 `tools/call` 的 `name`） |

**对 xylitol 的启示**：`get-something` 可以正确 dispatch——前提是客户端在暴露给模型的「公开名」之外，**另存** server id + 原始 MCP tool name，并在调用时用后者；不要用「拆 `__` / 还原 hyphen」当主路径。Codex 在 sanitize 碰撞时用 hash 后缀，进一步说明不能指望可逆。

---

## 1. Cursor

**公开形态（论坛一手引用）**：模型侧不是把 MCP 工具展成一长串 function name，而是走包装工具 `CallMcpTool`，参数含独立字段 `server`、`toolName`（以及实际需要的 `arguments`）。见：

- <https://forum.cursor.com/t/callmcptool-schema-omits-arguments-but-runtime-accepts-it/150043>
- <https://forum.cursor.com/t/allmcptool-schema-missing-arguments-field-causes-agents-to-call-mcp-tools-without-required-params/154996>

**连字符历史**：

- 2025-02：`nx-workspace` / `nx-docs` 在设置里可见，但 agent 不调用；改成 `nx_workspace` / `nx_docs` 后可用；Claude Desktop 对带 `-` 的同名工具正常。
  <https://forum.cursor.com/t/cursor-doesnt-detect-mcp-tools-with-in-name/53973>
- 同主题早期帖：用户与回复建议把 `-` 换成 `_`。
  <https://forum.cursor.com/t/cursor-refuses-to-use-my-mcp-server/51709>
- 2026-04：报 UI 警告「只能含 alphanumeric + underscore」；员工 **Dean Rie** 确认：`.` 未按 MCP spec 接纳，且会把 `act.click` 换成 `act_click`；**标题里的 hyphen「当前版本已支持」**，问题主要在 dot。
  <https://forum.cursor.com/t/cursor-incorrectly-filters-out-mcp-tool-names-containing-dots-and-hyphens-despite-being-valid-per-mcp-spec/157635>

**结论**：Cursor 用 **结构化 server + toolName** 避免拼串反解析；历史上 `-` 曾挡住 agent 选用；近期以员工回复为准，`-` 可用、`.` 仍会被 UI/规范化搅混。

---

## 2. OpenAI Codex（本地 `codex-rs`）

路径：`/home/l8ng/Projects/__straydragon__/codex/codex-rs/`（`mcp_tool_exposure.rs` 只做过滤/装配；命名在 `codex-mcp`）。

**模型可见名**：

- 前缀常量：`mcp` + 分隔符 `__` → 历史前缀 `mcp__`（`codex-mcp/src/mcp/mod.rs`：`MCP_TOOL_NAME_PREFIX` / `MCP_TOOL_NAME_DELIMITER`；`qualified_mcp_tool_name_prefix`）。
- `ToolInfo`：**raw** `server_name` + `tool.name`（发给 MCP）；**callable** `callable_namespace` / `callable_name`（给模型）。见 `codex-mcp/src/tools.rs` 模块注释与字段。
- `sanitize_responses_api_tool_name`：只保留 ASCII 字母数字与 `_`，其余（含 `-`、`.`）→ `_`（同文件 ~436–454；注释写 Responses 正则 `^[a-zA-Z0-9_-]+$`，实现比注释更严，**连 `-` 也洗掉**）。
- 测试 `test_normalize_tools_keeps_hyphenated_mcp_tools_callable`（`connection_manager_tests.rs`）：
  - server `music-studio`、tool `get-strudel-guide`
  - → `callable_namespace = mcp__music_studio`，`callable_name = get_strudel_guide`
  - **`tool.name` 仍为 `get-strudel-guide`**
- 碰撞：`basic-server` vs `basic_server` sanitize 后同形 → hash 消歧（同文件 `test_normalize_tools_disambiguates_sanitized_*`）。
- 装配：`McpHandler` 调用时用 `self.tool_info.server_name` + `self.tool_info.tool.name`（`core/src/tools/handlers/mcp.rs`），**不**从公开名反解析。
- `ToolName` 协议类型显式分 `namespace` + `name`（`protocol/src/tool_name.rs`）。

**不是** `mcp__server.tool`（点号）；常见拼串是 `mcp__{server}__{tool}`，且两侧都已 sanitize。

---

## 3. Claude Code / Anthropic + MCP spec

**MCP Tools（2025-11-25）** — <https://modelcontextprotocol.io/specification/2025-11-25/server/tools>

- Tool names SHOULD：长度 1–128；大小写敏感；字符仅 `A-Z a-z 0-9 _ - .`；server 内唯一。
- 聚合客户端 MAY 用 server 标识做前缀消歧；**勿依赖** `serverInfo.name` 全局唯一。

**Claude Agent SDK** — <https://platform.claude.com/docs/en/agent-sdk/mcp#tool-naming-convention>

- 模式：`mcp__{server}__{tool}`（例：`mcp__github__list_issues`）。
- Custom tools：`mcp__{server_name}__{tool_name}`（例：`get_temperature` @ `weather` → `mcp__weather__get_temperature`）。
  <https://platform.claude.com/docs/en/agent-sdk/custom-tools>

**Claude Code（插件 MCP）** — <https://code.claude.com/docs/en/mcp>

- `mcp__plugin_{plugin}_{server}__{tool}`；**保留** `A-Z a-z 0-9 _ -`，其它 → `_`。
- 例：`mcp__plugin_my-plugin_database-tools__query`（hyphen **留在**公开名里）。
- 配置侧 server 另有 `plugin:…:…` 作用域名，与 callable 字符串分离。

文档争议（Skills 的 `Server:tool` vs SDK 的 `mcp__…`）说明命名是 **客户端路由层**，不是 MCP wire：
<https://github.com/anthropics/claude-code/issues/18763>

---

## 4. 共同模式

1. **双下划线 `__` 作「段」分隔**（Claude / Codex 公开串；Cursor 则根本不拼进单一 function name）。
2. **Sanitize 常见但策略不一**：Codex 把 `-`→`_`；Claude Code plugin 文档保留 `-`；Cursor 对 `.`→`_` 有员工确认。
3. **结构化身份 + raw MCP name** 才是正确 dispatch：Codex `ToolInfo`；Cursor `server`/`toolName`；Claude 配置侧另有 scoped server 名。
4. **禁止把「可逆拆串」当主设计**：sanitize 碰撞与 hash 后缀已证不可逆。

---

## 给「get-something 能否 dispatch」的短答

可以。MCP wire 的 `tools/call.name` 本来就可以是 `get-something`。Cursor 用独立 `toolName`；Codex 模型侧见 `get_something`，调用仍传 raw `get-strudel-guide` 类名。两者都**不靠**从公开串还原 hyphen；Claude 的 `mcp__…__…` 同样是客户端前缀约定，wire 仍用 server 自己的 tool name。
