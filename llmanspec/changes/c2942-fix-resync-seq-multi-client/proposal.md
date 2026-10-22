---
depends_on: []
title: 修复 resync 回放、transient ModelSelect seq 与多客户端订阅绑定（P2）
needs_specs_change: true
branch: sdd/c2942-fix-resync-seq-multi-client
base_branch: main
base_sha: 46434c5dd88d8e4586e96028ee5a7d773ba16715
---

# 修复 resync 回放、transient ModelSelect seq 与多客户端订阅绑定（P2）

## Why

c2940 复核发现三处既有语义缺陷；它们不是本次 JSON-RPC 迁移引入的（`0da3e7c6` 旧代码同样存在），但新协议把它们暴露给第三方客户端：

- `bind_mux_to_session` 用 `mem::take` 把**所有**未绑定 mux 绑到当前 slot；多端 attach 时，A 的未 subscribe WS 可能被 B 的 subscribe 抢绑到 B 的会话。
- journal 回放只发给本次新绑定的 `bound` 连接；同一 WS 收到 `session/resync_required` 后 re-subscribe 时，该连接已在 `subscribers` 中，`bound` 为空，回放帧全部丢弃；客户端清空 UI 后收不到历史 tape。
- `sync_model_downlink` 广播 `session/event`（ModelSelect）时复用 `journal.max_seq()`（空 journal 时 0），不写 journal；这与“transient ModelSelect 不落 journal、不消耗 seq”的 r1926 以及“每会话单调唯一 seq”的 r1905 冲突；按 seq 去重的客户端会丢事件。

本 change 只修 P2；P0/P1 在 `c2941`。

## What Changes

- **订阅绑定定向**：WS 连接在 `serve_mux` 中把自己的 `MuxSink` 传给 `dispatch_text`；`bind_mux_to_session` 优先只绑定该连接（若已在 subscribers 中则复用其 conn id），POST `/rpc` 无连接身份时保留既有“绑定全部未绑定 mux”的兼容行为并在 spec 写明。
- **resync 回放闭环**：re-subscribe 时把已存在连接纳入回放目标；`replay_or_resync` 向该连接发送 `last_seq+1` 起的事件，再发 `session/subscribed`。
- **transient ModelSelect 独立通知**：新增下行 `session/model`（`{session_id, provider, model_id}`，无 seq、不写 journal）；`sync_model_downlink` 改发该通知；客户端 `decode_downlink` 识别并投影为 `XyEvent::ModelSelect`；r1915/r1926/r1803/r1905 同步。
- **上行 notification 语义**：live spec 明确“上行 notification 不执行、不应答”（与 design 一致），避免被误解为 JSON-RPC 标准语义已实现。
- **WS 超大文本帧**：inbound 帧超限/错误时显式发送 close 1009（或等价），不再静默断链。

## Capabilities

- `server-core`：订阅绑定、journal 回放、transient ModelSelect、下行方法表。
- `app-tui-host`：resync 重订与 ModelSelect 投影。
- `protocol-app`：上行 notification 语义说明。

## Impact

- 行为合约变更：`session/model` 新增；resync 回放目标改变；POST subscribe 兼容行为写明。
- 代码：`xylitol-serve` 的 `OccupiedHost`/`dispatch_text`/`bind_mux_to_session`/`sync_model_downlink`/`serve_mux`；`xylitol-protocol` 的 method/message；`xylitol-app-tui` 的 `json_rpc`/`remote`。
- 测试：serve 单测、app-tui 单测、BDD 的模型同步/再同步/多客户端场景。
- 不改变方法表其余语义、租约、幂等；不处理 P3 清理。
