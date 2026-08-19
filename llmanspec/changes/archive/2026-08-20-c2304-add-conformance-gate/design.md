# Design：符合性闸（chrome + 能力对齐 + 双 carrier）

实现顺序：**切片 2 → 3 → 1**（先红对拍，再扩表接线，最后 Echo 换成真 Host 双跑旧表）。方案 A（c2315）不在本票。

## 测试边界（seam）

| 切片 | 公共边界 | 不算数 |
|---|---|---|
| 2 | `XyEvent` ↔ `protocol::Event` 往返后 `apply_xy_event`；可选：真 Host `append_and_push` → mux 订阅 → 同一投影 | `activity_fold` 直接 `tool_start(..., path)` |
| 3 | `POST /api/{method}` + `XyRemoteDriver` 对上表新 unary；InProcess Driver 对照 | 旧 `/api/v1` REST |
| 1 | `InProcessClient`（真 HostState，非 Echo）与 `HttpWsClient` 同表 | 面本地 Quit/键/画 |

BDD：扩 `server-core` 已有 feature 能挂的 GWT；`protocol-app` 现无 `.feature`，新条款可先 `feature: false` + Rust 单测（对齐该 spec 现状）。禁止另开脱离 feature 的 CLI 子进程协议。

## 切片 2：工具 chrome 过线

根因：`Event::ToolStart { id, name }` 丢掉 `XyEvent::ToolExecutionStart.args`；还原 `args: Null`。`read` 的 End 正文不是带 path 的 JSON，无法回填。`MessageUpdate` 丢掉 `message`（toolCall）会饿死流式意图 chrome。

- `Event::ToolStart` 增加 `args`（serde default；缺字段不 panic）。`to_wire_event` / `try_from` 保留。Pre-0.0.1：**禁止**双解析旧形状。
- 先写失败测：带 `path` 的 Start → wire 往返 → `args_preview` MUST 含路径，MUST NOT 停在 `Read ...`。
- 流式 path 仍饿死时，同切片让 `MessageUpdate` 携带足够 toolCall（或整段 `message`），仍不新开 Event 变体。
- specta 重生 `bindings.ts`。

## 切片 3：登记 TUI 已用缝

c2302「本票不扩表」对本票作废。禁止用 REST 冒充（改写 `sr-st1`）。

**Session / store（进 `Command` + `dispatch`，payload 用现有 Driver DTO，不另写平行类型）：**

`session_tree` · `travel_session_tree` · `append_entry_label` · `list_sessions` · `load_session_entries` · `new_session` · `get_session_name` · `set_session_name` · `set_session_name_for` · `delete_session`

只读（list / tree / load_entries / get_name）不占写者。其余占写者。

**进程级（与 `host.describe` 同类，不绑单一 session 槽）：**

- `reload`：重装 skills / MCP / prompt（面本地键位/主题仍 TUI）。进行中 TUI Esc → 已有 `abort` 取消该 Host 上的 reload token（对齐 ath28 协作取消，不另开 cancel 方法）。
- `loaded_resources`：只读 MCP/skills 快照。Remote MUST NOT 再返回 `Default` 空快照。

**`leaf_entry_id`：** 并进现有 `get_state` 快照，不新开 unary。Remote 禁止恒 `None`。

`XyRemoteDriver` 对这些走 unary；禁止 `unsupported` 与 reload 默认 no-op。

仍保留：`load_debug_scene`、clipboard、`persist_project_trust`、bash 直播、`queue_stats`。

## 切片 1：Echo → 真 Host

`InProcessClient::echo()` 不得过闸。进程内客户端 MUST 持 `writerToken`，打同一 `handle_unary`。旧表（c2290 已登记行）与切片 3 新行同一套双跑。未知方法失败。

## live specs（Branch binding 后）

- `protocol-app`：ToolStart args 往返；方法表新行；reload / loaded_resources。
- `server-core`：`sr-st1` 改为经已登记 unary；Host 接线。
- `app-tui-bridge`：wire 往返后 path chrome 与直播同源（atb13 的 attach 推论）。
