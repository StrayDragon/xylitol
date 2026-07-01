---
change_id: c345-unify-server-protocol
title: 让 server/ws.rs 靠拢 protocol SSOT，消除与 rpc.rs 的协议分裂
status: proposed
priority: 345
depends_on:
  - c325-add-app-tui-spec
author: agent
---

# c345-unify-server-protocol

## Why

xylitol 声明的意图是「rpc 与 server speak 同一个 `protocol::` SSOT」（`rpc.rs` 模块注释原文：*"a future WebSocket/REST transport and a server will speak the same types without touching this file"*）。但**现状并非如此**：

- `rpc.rs` speaks `protocol::{Command, Event}`（SSOT）。
- `server/ws.rs` 另造了 `ClientFrame` / `ServerFrame`（`ServerHello`/`Ack`/`Event`/`ResyncRequired`/`Subscribe` 等）这**第二套**帧类型，而非直接复用 `Command`/`Event`。

这是与两个对标项目的主要差距：

### 调研证据

- **codex 的做法**：`app-server-protocol` 是单一协议 crate，in-process / stdio / ws / unix-socket **四种传输 speak 同一个 JSON-RPC 协议**。in-process 路径甚至「reuse the same JSON-RPC result envelope internally」（`app-server-client/README.md:41-43`）。codex 证明「一个协议服务多传输」是可行且已被工业验证的。
- **pi 的做法**：`RpcCommand`/`AgentSessionEvent` 这一套 JSONL 协议，**同时**用于 stdio-RPC 子进程和 orchestrator 的 unix socket——orchestrator 是「在 stdio-RPC 协议之上的多路复用器」，而非另造协议。
- **xylitol 现状**：`protocol::Command/Event` 已是 SSOT，但 server 没有真正消费它，而是套了一层 `ServerFrame`/`ClientFrame`。这导致「同一语义两套类型」，维护时易漂移。

### 战略价值

统一协议后，c335 的共享 `dispatch` 能**同时**服务 rpc（stdio）和 server（WS）——只需在 server 端把收到的 `Command` 直接交给共享 dispatch，无需在 `ServerFrame` 与 `Command` 间翻译。这是「三面并存」架构下，命令行为跨传输一致的最终闭环。当前 server 的 `ReverseRpcGateway`（审批/提问反向 RPC）是 WS 传输特有，保留在 server 层不进共享 dispatch（与 c335 一致）。

## What Changes

1. 让 `server/ws.rs` 的帧类型对齐 `protocol`：`ClientFrame` 的 `Subscribe`/命令变体映射到 `Command`；`ServerFrame::Event` 直接承载 `protocol::Event`（而非中间类型）。
2. 保留 WS 特有的传输控制帧（`ServerHello`/`Ack`/`ResyncRequired`）——它们是连接生命周期管理，不属命令语义，留在 server 层合理。
3. server 的命令处理改调共享 `dispatch`（依赖 c335），消除 server 内自带的命令执行逻辑。

## Capabilities

- `app-protocol`（修改）：声明 `Command`/`Event` 是所有传输（stdio/WS）的统一协议。
- `server-ws`（修改）：帧类型靠拢 protocol SSOT。
- `server-reverse-rpc`（不变）：ReverseRpcGateway 是 WS 传输特有，保留。

## Impact

- **受影响代码**：`src/app/server/ws.rs`（帧类型重构）、`src/app/server/rest.rs`（可能简化）、`src/app/core/driver.rs::RemoteDriver`（消费统一协议）。
- **受影响规范**：`app-protocol`、`server-ws`。
- **风险**：中。触及 server（已验证）的 WS 协议。必须有回归测试证明 server 既有 WS 行为（subscribe/event 流/resync）不变。

## 反降级护栏（防止本变更被降级为「只新增映射但 server 仍用 ServerFrame」）

- [ ] `server/ws.rs` 的命令处理 MUST 直接消费 `protocol::Command`（经共享 dispatch），MUST NOT 保留平行的 `ClientFrame` 命令变体。
- [ ] 传输控制帧（`ServerHello`/`Ack`/`ResyncRequired`）MAY 保留为 WS 特有（属连接生命周期，非命令语义）。
- [ ] server 既有 WS 行为（subscribe/event 流/resync 检测）MUST 回归通过。
- [ ] `RemoteDriver` MUST 能在统一协议下端到端跑通一个 prompt 的事件流。
- [ ] 本变更 MUST NOT 砍掉 rpc（c325 证据 C：三面并存）。
