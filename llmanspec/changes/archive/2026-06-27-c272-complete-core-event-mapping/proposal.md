---
depends_on:
  - c271-add-server-integration
---

# c272-complete-core-event-mapping

> **状态**：设计提案（2026-06-26）。聚焦已有功能逻辑的完备性，不涉及 server/network 新功能。

## Why

c270 + c271 完成了 server 装配焊接，但暴露了核心功能逻辑层的历史欠账：

1. **AgentEvent 到 protocol::Event 的映射只覆盖了 ~60% 的变体**——TurnStart/TurnEnd/MessageStart/MessageEnd/MessageUpdate/ToolExecutionUpdate/CompactionEnd 一共 7 个变体被遗漏，RPC/WS 客户端收不到这些事件。
2. **HC-2 port 层有名无实**——`SessionStore`/`EventSink` trait 定义了、实现了，但 `AgentSession::new` 仍接受具体 `SessionManager`，`Agent::with_ports` 的 port 参数以下划线前缀弃用。
3. **RPC 模式效率低**——每个命令都重建 `ToolRegistry` + `SessionManager` + `Agent`。
4. **print 模式事件处理不完整**——仅处理了 7/14 个 `AgentEvent` 变体。

## What Changes

### P1 — 补全 AgentEvent → protocol::Event 映射

- `protocol.rs::Event` 增补：
  - `TurnStart { turn_index: u32 }`
  - `TurnEnd { turn_index: u32 }`
  - `MessageStart { role: String }`
  - `MessageEnd { role: String }`
  - `MessageUpdate { text: String, thinking: Option<String> }`
  - `ToolExecutionUpdate { id: String, output: String }`
  - `CompactionEnd`
- `server/rest.rs::agent_event_to_protocol()` 补全所有变体映射
- `interactive/driver.rs::proto_to_agent()` 补全所有变体映射
- `interactive/rpc.rs::run_prompt()` 的事件匹配补全新变体

### P2 — AgentSession 实际消费 SessionStore/EventSink ports

- `agent/session/io.rs::SessionIO`：从接受 `SessionManager` 改为接受 `Arc<dyn SessionStore>`
- `agent/session/mod.rs::AgentSession::new`：接受 `store: Arc<dyn SessionStore>` + `sink: Arc<dyn EventSink>` 替代具体 `SessionManager`
- `agent/facade.rs::Agent::with_ports`：移除 `session_mgr: SessionManager` 参数（port 已携带），去掉 `_store`/`_sink` 下划线前缀
- 更新所有调用点（cli、rpc、runtime、tests）

### P3 — RPC 模式缓存 Agent

- `interactive/rpc.rs::RpcState`：持有一个缓存的 `Agent`，而非每次重建
- `build_session()` → `ensure_agent()`：懒初始化 + 复用，仅 session_id/模型选择等参数变化时才替换
- `EventBus` 注入在 Agent 构造时一次性完成

### P4 — print 模式事件完备

- `interactive/print.rs::run_print`：补全所有 `AgentEvent` 变体的处理（TurnStart/TurnEnd/MessageStart/MessageEnd/MessageUpdate/ToolExecutionUpdate/CompactionEnd）

## Capabilities

- `interactive-protocol`（modify）：增补 7 个 Event 变体
- `agent-runtime`（modify）：AgentSession 消费 port 对象
- `interactive-client`（modify）：RPC Agent 缓存 + print 模式完备
- `layer-architecture`（modify）：移除 arch_guard 中的 rpc.rs/print.rs 例外标记

## Impact

- **协议变更**：`protocol::Event` 新增 7 个序列化变体——向后兼容（serde `tag` + `snake_case`，新 client 收到未知 tag 可忽略）
- **性能提升**：RPC 模式避免每次命令重建 Agent（文件 I/O + 工具注册）
- **依赖缩减**：消除 `AgentSession` 对具体 `SessionManager` 的直接依赖，完成 HC-2 装配
- **测试**：现有 BDD 88 场景不变，新增的事件变体通过序列化测试验证
