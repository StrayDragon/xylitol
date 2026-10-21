# 会话准备门与根包收口

## 准备缝

`session/prepare` 是 wait/timeout 瀑布事件，不是 shell hook，不改 `agent/pre-step` 的 fail/continue。ReAct 在拼 prompt / `tools[]` 前 wait；切会话 / resume / idle `/reload` 只 emit 开火（unary 不阻塞，对齐 mcp8）。mcp 插件监听：连 MCP、武装工具、超时武装子集。后续 `c1960` 在同一事件上换「顶栏就绪即可」监听器。

TUI 不认 rutis。Assembling / connecting 仍走信封 loaded-resources 快照。

## 正确性与 KV cache

不改写既有 turn（前缀 cache）。准备指纹/正文相对上次有变时，持久化 Env 行（`session_env` 同族：有变 append，投影为 user），插在下一真实 user 之前，让模型就近看见增量。无变不追加。整份 system 仍走 generate options，不写成 user（`r1039`）。`tools[]` 真变时出线 schema 仍可能单独打 cache，那是正确性。

禁止插入「MCP ready」假聊天行（mcp8 仍在）。准备 Env 不是那类宣告。

## 驱动包装

`XyComposedDriver` Deref 到 `RuntimeDriver` 会绕过端口。本 change 拆掉包装：reload / todo 挂到 runtime 进程内驱动或核心插件，embed / BDD 改用 `RuntimeDriver`。不把包装压成「只删 Deref 留类型」。

## occupancy 胶水

`in_process_client` 与 dual-rail 必须同时碰到 Host 与产品 TUI 客户端。crate 图禁止 kernel/serve 依赖 app-tui。产品 TUI 已 attach-only。胶水下沉 `tests/`；CLI 直调 kernel occupancy；根包不再 `src/wiring`。

## 不选

- 本 change 做 `tool_search`：延期票，且消不掉准备门。
- 切会话 unary 内 wait：违反 mcp8。
- 只改 system 前缀承载 MCP 增量：注意力远、整段 cache 更易废。
