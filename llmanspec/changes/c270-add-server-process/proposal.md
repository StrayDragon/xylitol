---
depends_on:
  - c260-refactor-domain-architecture
  - c265-add-server-runtime
---

# c270-add-server-process

> **状态**：设计提案（2026-06-26）。仅 proposal + design，尚不包含 spec deltas 和 tasks。从 c260/c265 拆分而来——c265 已完成 HC-2 端口（SessionStore/EventSink）和 Driver 抽象，为 server 托管铺好了底座。本 change 承载 server 进程化本身——新功能、新依赖、新状态机，与重构性质的 c260/c265 不同。
>
> **前置依赖**（已全部 archive）：`c260-refactor-domain-architecture` → `c265-add-server-runtime`。

## c265 未完成项（c270 必须先落地）

c265-add-server-runtime 的 tasks.md 全勾选，但部分项仅**部分完成**。c270 在推进 server 前必须先完成这些，否则 server 装配会断裂：

| c265 任务 | 实际完成度 | c270 需补完 |
|---|---|---|
| T7 facade HC-2 | `with_ports` 构造器已加，但 `Agent::run` 的 `_session_id` 参数兼容保留。server 需要干净的 `run(prompt)` | 移除残留的 `run_with_id`，`run` 完全负责 session_id 内部生成 |
| T8 cli 组合根 | cli 仍用 `Agent::new(agent_session)` 而非 `Agent::with_ports`。server 需要纯 port 构造路径 | cli 改用 `Agent::with_ports`（model_registry/tool_registry 已就位） |
| T9 rpc 接收 Driver | rpc.rs 的 `run()` 仍接收 `AgentSession` 而非 `Driver`/`Agent` | rpc 改用 `InProcessDriver` 或直接 `Agent`；RPC 状态机不应触及 `AgentSession` 内部 |
| T12 interactive 全量迁移 Driver | print 和 cli 已迁移，但 rpc 未动 | rpc 完成迁移 |
| T14 arch guard interactive 断言 | 实际未添加测试——仅更新了 NOTE 说明 | 添加 `interactive_only_from_driver` 测试（仅 driver.rs 可 import agent/infra） |

此外，server 进程化（HTTP/WS 框架、锁、journal、反向 RPC、RemoteDriver、CLI 子命令、BDD）是全新工作，此前在 c260/c265 中仅有 `defer → c270` 的占位。

## Why

c260 + c265 完成了以下铺垫：
- protocol SSOT（Command/Event enum）
- 薄 agent + infra 运行时域 + port 注入（SessionStore/EventSink）
- Driver 抽象（InProcessDriver/RemoteDriver）

但 xylitol 目前仍是**单进程单用户**形态：agent 在 cli 进程内直接构造，交互层在前端进程中共存。这无法满足以下场景：

1. **核心常驻**：agent 进程启动后持续运行，接受多轮 prompt、处理后台任务（compaction/planning），不因 client 断开中断。
2. **多 client 共享**：一个 server 服务多个 client（tui + web + CI），各 client 可随时连接/断开/重连。
3. **交互可离线**：client 断开期间 server 继续工作，重连后补齐事件序列。client 可换设备重连同一 session。
4. **一控多 / 多控一**：一个 client 控制多个工作区 server；多个 client 协作同一 session。
5. **进程化部署**：`server run` 子命令后台进程化，支持 `server install`（launchd/systemd）和 `server stop`。

本 change 也是 kimi-code 架构（agent-core + server + protocol + apps）在 xylitol 单 crate 下的最终落地。

## What Changes

### P1 — HTTP/WS 框架引入 + 单实例锁
- `Cargo.toml` 加 axum（HTTP）和 tokio-tungstenite（WS）——与既有 tokio 生态契合，非重型栈
- `server/lock.rs`：单实例锁（`O_CREAT|O_EXCL` 文件锁 + 端口关联），第二实例 `ServerLockedError`
- `server/port_retry.rs`：端口忙 `port + 1` 重试（上限 PORT_RETRY_LIMIT），锁文件更新真实端口

### P2 — protocol 补齐 + REST 路由
- `protocol.rs` 增补 `Command::Subscribe`（订阅 session 事件流）、`Event::Subscribed`、envelope `{code, msg, data, request_id}` 类型
- `server/rest.rs`：`/api/v1` 下挂 control 路由：
  - `POST /api/v1/session/{id}/run` — 提交 prompt
  - `DELETE /api/v1/session/{id}` — 取消
  - `POST /api/v1/session/{id}/model` — 切换模型
  - `GET /api/v1/session/{id}/events?seq=N` — 长轮询事件（fallback）
  - `GET /api/v1/healthz` — 健康检查
  - `GET /api/v1/openapi.json` — API 文档（可选）

### P3 — WS 连接 + 事件流
- `server/ws.rs`：`server_hello`/`ack`/`event`/`resync_required` 帧协议
- 每 session 维护单调 `seq`（`AtomicU64`），每 append event 自增
- `subscribe(session_id, last_seq)`：回放 `last_seq+1..` 之后的事件序列，然后接实时流
- 环形/截断 journal：保留最近 N 条（默认 10000），超出截断时新 client `subscribe` 推 `resync_required`

### P4 — server 运行时装配
- `server/runtime.rs`：第二个组合根（第一个是 `interactive/cli/mod.rs`）
  - 构造 `Arc<dyn SessionStore>`（`SessionManager`）+ `Arc<dyn EventSink>`（`EventBus`）
  - 从配置加载 ModelRegistry、ToolRegistry，注入 agent ports
  - 暴露 `RunningServer` 句柄（graceful shutdown）

### P5 — 反向 RPC
- protocol 增补 `Command::ApproveTool` / `Command::AnswerQuestion`
- server 侧：agent 发出 tool approval request 时，匹配对应 session 的 `call_id`，通过 WS 推给该 session 的持有 client，挂起 turn
- client 侧：`interactive/driver.rs::RemoteDriver` 接收反向 RPC 事件，等待用户应答后回传 `ApproveTool`
- 多 client 连同一 session 时**第一个应答生效**（v1，同 c260 design §3）

### P6 — RemoteDriver + CLI 子命令
- `interactive/driver.rs` 加 `RemoteDriver`（WS + REST 双通道）
  - `send(cmd)`: control 命令走 REST，返回 `Result<Value, Error>`
  - `subscribe(session_id, last_seq)`: 事件流走 WS
  - 实现 `Driver` trait 以复用 `interactive/print.rs` 等 client 代码
- CLI 加子命令 `xylitol server run` / `server install` / `server stop`

### P7 — BDD + 集成测试
- 更新 `rpc.feature`：适配 protocol envelope
- 新增 server 重连场景：`server.feature`（订阅 → 断连 → 工作 → 重连 → 补齐）
- 新增反向 RPC 场景：`approval.feature`（工具执行 → 审批请求 → 用户审批 → 恢复）

## Capabilities

- `server-runtime`（从占位 add→full）：锁/port-retry/runtime 装配/healthz
- `interactive-protocol`（modify）：增补 Command::Subscribe + envelope + error-code
- `interactive-client`（modify）：加 RemoteDriver
- `server-ws`（**新增**）：WS 帧协议 + seq + journal + resync
- `server-rest`（**新增**）：REST `/api/v1` 路由 + envelope
- `server-reverse-rpc`（**新增**）：approval/question gateway
- `cli-entry`（modify）：增补 server run/install/stop 子命令
- `session-persistence`（modify）：journal 访问接口（如必要）
- `layer-architecture`（modify）：arch_guard 加 interactive 断言（c260 NOTE 待启用项）

## Impact

- **新依赖**：axum（HTTP 框架）、tokio-tungstenite（WS）、可能 `serde_path_to_error`（REST 错误处理）。
- **依赖追加策略**：`axum` 选默认 feature（最小集），`tokio-tungstenite` 选 `connect` + `server`。非重型栈，与既有 tokio/tower 生态契合。
- **新状态机**：WS 连接生命周期（`hello` → `ack` → 持续 `event` → 断连 → 按需 `resync`），配 property-based 测试。
- **行为新增**：server 进程、`server run/install` 子命令、远程 Driver。
- **风险**：高。WS 状态机（断连/重连/seq race）是最大难点。反向 RPC 的 call_id 匹配需原子性保证。

## YAGNI Boundaries

- 不做 server 间分布式协调（每 server 独立核心，扩展 = 多实例）
- 不做 protocol 版本协商（v1 冻结现有 Command/Event，新 variant 加`#[serde(other)]`兜底）
- 不做 web/tui client 实现（c270 只做 server + RemoteDriver，client 本身后续独立）
- 不做 OAuth / provider attribution headers（项目 scope 1.0 前不支持）
- 不做 server 远程日志/监控（按需后续）
