---
depends_on:
  - c270-add-server-process
---

# c271-add-server-integration

> **状态**：设计提案（2026-06-26）。补完 c270 的装配 gap——c270 完成了组件（锁/端口重试/WS帧/journal/反向 RPC gateway/协议扩展），但 **Agent 未挂进 REST 路由，WS 未挂载，RemoteDriver 是占位，所有端点返回 501**。本 change 做最后的焊接。

## Why

c270-add-server-process 交付了以下可独立验证的组件：

| 组件 | 状态 |
|---|---|
| `ServerLock`（原子文件锁） | ✅ 单元测试覆盖 |
| `port_retry`（端口重试） | ✅ 单元测试覆盖 |
| `protocol` 扩展（Subscribe/ApproveTool/Envelope） | ✅ 类型就位 |
| `EventJournal`（环形 buffer + seq） | ✅ 单元测试覆盖 |
| `ReverseRpcGateway`（call_id→oneshot） | ✅ 单元测试覆盖 |
| `ServerFrame`/`ClientFrame` 枚举 | ✅ 序列化测试 |
| `Agent::with_ports` 接受 port 对象 | ✅ cli/rpc 迁移完成 |
| `RunningServer`（graceful shutdown 句柄） | ✅ 存在 |

但装配接缝处是空的：

```
xylitol server run --port 9090
  ✓ 锁文件创建
  ✓ HTTP 服务器启动
  ✗ GET /api/v1/healthz → "ok"
  ✗ POST /api/v1/session/x/run → "not yet implemented"
  ✗ WS upgrade → 没有路由
  ✗ RemoteDriver → 发请求得到 501
```

本 change 做最后的焊接：**Agent → AppState → REST handler → WS upgrade → RemoteDriver**。

## What Changes

### P1 — Agent 挂进 AppState，REST 路由功能化

- `server/runtime.rs`: `AppState` 持有 `Arc<Mutex<Agent>>` 而非空结构体；`RunningServer` 保持句柄
- `server/rest.rs`:
  - `POST /api/v1/session/{id}/run` — 从请求体提取 prompt，调用 `agent.run(prompt)`，返回 `session_id` + 启动事件流
  - `DELETE /api/v1/session/{id}` — 调用 `agent.abort()`，取消 token
  - `GET /api/v1/session/{id}/events?seq=N` — 拉取事件（长轮询 fallback，配合 journal）
- 移除所有 "not yet implemented — runtime assembly in progress" 桩

### P2 — WS upgrade 挂载

- `server/rest.rs` 或新建 `server/router.rs`: 用 `axum::routing::get` + ws upgrade handler 挂 `/ws/session/{id}` 或 `/api/v1/session/{id}/ws`
- 握手时序：client 发 `Subscribe` → server 回 `ServerHello + Ack` → 事件流推送
- 透传 `ReverseRpcGateway`：收到 `ApproveTool`/`AnswerQuestion` 帧时通过 gateway 消费

### P3 — RemoteDriver 完整化

- `interactive/driver.rs::RemoteDriver`:
  - `run()`: REST POST 提交 prompt + WS connect 订阅事件流（tokio-tungstenite connect）
  - 事件流持续推送直到 `AgentEnd`
  - 支持断连重试（有限次）
  - `abort()`: REST DELETE + WS close

### P4 — server stop 进程间信号

- `server stop` 子命令：往锁文件里的 PID 发送 SIGTERM，而非只删锁文件

### P5 — 集成测试

- 新增 `tests/bdd.rs` step definitions 对应 `server.feature` 场景：
  - 启动 server → healthz 检查 → 提交 prompt → 接收事件 → 停止
- 新增 `tests/bdd.rs` step definitions 对应 `approval.feature` 场景：
  - 涉及 FakeProvider + approval_required 工具

## Capabilities

- `server-runtime`（modify）：AppState 持有 Agent
- `server-ws`（modify）：WS upgrade handler + 事件推送
- `interactive-client`（modify）：RemoteDriver WS 流式
- `cli-entry`（modify）：server stop 发 SIGTERM
- `bdd-tests`（modify）：server/approval 集成场景

## Impact

- **新依赖**：无（axum + tokio-tungstenite 已在 c270 引入）
- **行为变化**：server 从 "能启动但啥也做不了" 变成 "真正能跑 prompt"
- **风险**：中。WS 状态机（断连/重连/seq race）是唯一天然难点。反向 RPC 的 call_id 匹配在 WS 多 client 场景下需确认原子性。
- **测试增益**：从 0 集成测试 → 有 server BDD 场景覆盖核心路径

## YAGNI Boundaries

- 不做多 session 管理（server 启动一个 session，后续扩展）
- 不做 client 认证（同 c270 原则）
- 不做 WS 自动重连的指数退避（有限次线性重试足够 v1）
- 不做事件 journal 的持久化到磁盘（当前环形 buffer 在内存中）
