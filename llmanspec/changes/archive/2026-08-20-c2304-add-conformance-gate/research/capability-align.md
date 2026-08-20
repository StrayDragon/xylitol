# attach 路径：TUI 已用、方法表未登记的 Driver 缝

> 2026-08-20。用户要求并进 c2304：session 树 / 新会话 / 列表 / reload / MCP 快照登记进 unary 并接到 Host；**对齐后再做方案 A（c2315）**。c2302 曾写「不扩方法表」；本票显式改口。

## 产品面已经在调、Remote 却 unsupported / 空快照

产品 TUI 经 `XyDriver` 调这些缝（InProcess 有实现）。`XyRemoteDriver` 现状：

| Driver 缝 | TUI 用途 | Remote 现状 |
|---|---|---|
| `session_tree` / `travel_session_tree` | Esc 树、Enter travel | `unsupported` |
| `append_entry_label` | 树标注 | `unsupported` |
| `leaf_entry_id` | `/session-fork` 当前叶 | 恒 `None` |
| `list_sessions` | resume 面板、退出 hint、editor 历史 | `unsupported`（文案：not a v1 unary） |
| `load_session_entries` | resume / 新会话种子历史 | `unsupported` |
| `new_session` | `/session-new` | `unsupported` |
| `get_session_name` / `set_session_name` / `set_session_name_for` | 显示名 | `unsupported` |
| `delete_session` | resume 面板删 | `unsupported` |
| `reload_runtime` | `/reload`（host 侧重装 skills/MCP/prompt） | **trait 默认 no-op `Ok`**，不是 Err |
| `loaded_resources_snapshot` | 头卡 / `/mcp` | **返回 `Default` 空快照**，不是 Err |

`switch_session` / `fork` / `get_messages` 已在 v1 unary。树与列表是另一组缝：c2290 方法表草稿标「保留」；`server-core` `sr-st1` 禁止用旧 `/api/v1` REST 冒充——正确落地是 **登记 unary**，不是再开 REST。

## 进程范围 vs session 槽

c2302：Host 进程共享一份 RuntimePorts；**reload 打在这份基线**，再按槽物化。MCP 快照同理（监听器级资源，不是某个 session 私有）。

session 树 / list / new / rename / delete / load_entries / travel / leaf 是 **session 或 store 级**，走写者租约或只读 unary（list / tree / snapshot 只读；new / delete / travel / reload / rename 写）。

## 本票必须一起登记的伴生缝

只登记用户点名的五个名字，面板仍残：travel、load_entries、rename、delete、leaf。本票把它们算同一切片。

仍保留（不进本票）：`load_debug_scene`、clipboard（面本地）、`persist_project_trust`、bash 直播增量、`queue_stats`（队列条已有 `QueueUpdate` 下行）。

## 闸

双 carrier：InProcess vs HttpWs 对上表每个新 unary 跑通（含拒绝 / 缺 session / 写者冲突）。未知旧名仍失败。specta 重生 `bindings.ts`。
