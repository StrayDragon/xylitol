---
depends_on: []
---

# 下行补 session/resources：MCP chrome 走 mux，不进 journal

产品 TUI attach 后，MCP / skills 头卡不能靠 TUI 每 tick unary `loaded_resources`。Host 在写者侧本地 poll，脏了才推一帧 chrome 快照。本票把这条已经落地的方法收进 `server-core` 合约；落实走 propose（默认分支不得改 live specs）。

## Why

`server-core` 的 `w1` 产品下行 method 目前 MUST 含 `session/event`、`session/subscribed`、`session/resync_required`。MCP 连接态刷新不是回合磁带：塞进 `session/event` 会占 seq、进 journal，断线重放会把 chrome 当实况事件。TUI 16Hz unary 则在 Assembling 时反复打网。

需要一条 **非 journal** 的 mux 下行，专推 LoadedResources 快照。未知下行对旧客户端已是忽略（`Some(Ok(_))`），加法兼容。

## What Changes（草案，未拍板）

- 产品下行 method 表 **MUST** 含 `session/resources`（与 `session/event` 并列，不是 Event 变体）。
- payload：`session_id` + 当前 LoadedResources 快照。**MUST NOT** 消耗 journal seq，**MUST NOT** 写入 EventJournal。
- Host：写者 `poll_mcp_bootstrap` 在 Host 进程内跑；仅 dirty 时向该 session mux 广播。TUI tick **MUST NOT** 为同一刷新再 unary `loaded_resources`（可在收到帧后读本地 cache）。
- 旧客户端：不认识该方法则忽略；不要求 bump protocol。
- 对拍：Assembling 期间 MCP 头卡从 pending → connected 不靠 16Hz unary；journal 重放不得复播 chrome 帧。

## 开放决策（propose 时深挖）

- `w1` 是扩 MUST-contains 列表，还是另开 `w8` 专写 chrome 下行（与 journal 事件分岔更清晰）。
- snapshot 形状是否钉死为 `loaded_resources` unary 的同一 Value，还是收窄字段。
- 是否允许同连接多客户端各收一份（广播 vs 写者专属）。

## Capabilities（拟）

- `server-core` / `protocol-app`（下行方法表、非 journal 约束）
- `app-tui-host`（tick 只读 dirty/cache，不轮询 unary）

## Impact

Attach TUI 的 MCP 头卡与同进程一致，且不把 chrome 污染进回合磁带。

## 非目标

改 ReAct / 工具闸；把 skills 文件内容塞进下行；c2307 冷恢复 journal 回放；bump `PROTOCOL_VERSION`。
