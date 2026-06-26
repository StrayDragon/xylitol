# c271-add-server-integration — Design

> 前置设计：c270 design.md（锁/端口重试/WS帧/journal/反向RPC基础）。本 change 不做新组件引入，只做装配焊接。

## 1. AppState 设计

```rust
pub struct AppState {
    pub agent: Arc<Mutex<Agent>>,
    pub journal: Arc<Mutex<EventJournal>>,
    pub gateway: Arc<ReverseRpcGateway>,
}
```

`Arc<Mutex<Agent>>` 保证路由 handler 能安全访问 agent（axum handler 要求 `State` 是 `Clone + Send + Sync`）。`EventJournal` 共享引用让 REST 的 get_events 和 WS 的事件推送使用同一份 seq 空间。

## 2. WS upgrade 位置

挂载在 `GET /api/v1/session/{id}/ws`。使用 `axum::extract::ws::WebSocketUpgrade`（axum 内置 WS 支持，无需额外 crate）。

握手时序：
```
Client → Server:  {"type": "subscribe", "session_id": "s0", "last_seq": 0}
Server → Client:  {"type": "server_hello", "version": "1.0"}
Server → Client:  {"type": "ack", "seq": 0}
Server → Client:  {"type": "event", "session_id": "s0", "seq": 1, "event": {...}}
...
```

## 3. RemoteDriver WS 路径

`RemoteDriver::run()` 使用 `tokio-tungstenite` 的 `connect_async` 连接 server WS：
1. POST prompt 到 REST（触发 agent 开始工作）
2. 连接 WS，发送 Subscribe
3. 接收 ServerHello + Ack
4. 持续接收 Event 帧，yield 到 EventStream
5. 遇到 AgentEnd 或断连时结束

## 4. server stop 信号

读锁文件 JSON → 提取 pid → `nix::sys::signal::kill(Pid, SIGTERM)` → 等待最多 5 秒 → 删锁文件。跨平台：Unix 用 `nix` crate，Windows 暂不支持（返回错误信息）。

## 5. 集成测试策略

使用 `wiremock` 或真实子进程：`std::process::Command` 启动 `cargo run -- server run --port X`，用 reqwest 发 HTTP 请求验证。这是端到端测试，不 mock 任何内部组件。
