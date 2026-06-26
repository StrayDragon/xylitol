# Tasks: c270-add-server-process

> **工作说明**：本 change 包含两类工作——(A) **补完 c265 未完成项**（T1-T4，c265 tasks.md ✅ 但代码实际未完成），(B) **server 进程化本身**（T5-T21）。所有前置已在 proposal.md 中透明声明。每阶段后全测试套件绿灯才进下一阶段。

## P0 — 补完 c265 未完成项（server 装配基础）

- [x] T1 改 `interactive/cli/mod.rs`：cli 组合根从 `Agent::new(agent_session)` 改为 `Agent::with_ports(store, sink, ...)`，port 在组合根构造
- [x] T2 改 `interactive/rpc.rs`：`run()` 改为接收 `Driver` / `Agent`（port 已由调用者注入），移除 `build_session()` 和 `AgentSession` 直接引用
- [x] T3 在 `src/tests.rs` 新增 `interactive_only_from_driver` 测试：验证 `interactive/`（除 `driver.rs` 外）不 import `crate::agent` 或 `crate::infra`
- [x] T4 验证：build + nextest + clippy + BDD + arch_guard 全绿

## P1 — 新依赖引入 + 单实例锁

- [x] T5 `Cargo.toml` 加 `axum`（default-features）、`tokio-tungstenite`（features = ["connect", "server"]）、`tower-http`（features = ["cors"]）
- [x] T6 新建 `server/lock.rs`：`ServerLock::try_acquire(lock_path)` 用 `O_CREAT|O_EXCL` 原子创建锁文件（写入 port/pid/hostname JSON），`ServerLock` drop 时删除锁文件；`ServerLockedError` 返回现有锁文件内容
- [x] T7 新建 `server/port_retry.rs`：端口忙时 `port + 1` 重试（上限 10），更新锁文件端口号

## P2 — Protocol 补齐 + REST 路由

- [x] T8 `protocol.rs` 增补 `Command::Subscribe {session_id, last_seq}`、`Event::Subscribed {session_id, seq}`、REST envelope 类型（`{code, msg, data, request_id}`）及 typed error-code 枚举
- [x] T9 新建 `server/rest.rs`：`/api/v1` 下挂路由——`POST /session/{id}/run`、`DELETE /session/{id}`、`POST /session/{id}/model`、`GET /session/{id}/events?seq=N`、`GET /healthz`

## P3 — WS 连接 + 事件流

- [x] T10 新建 `server/ws.rs`：`ServerFrame`/`ClientFrame` 枚举（含 handshake `ServerHello`+`Ack` → `Subscribe` 顺序）；`subscribe(session_id, last_seq)` 回放 + 实时流接口
- [x] T11 每 session 单调 seq（`AtomicU64`）+ 环形 journal（默认 10000 条），journal 截断时推 `ResyncRequired`；`read_recent(session_id, limit)` 接口

## P4 — Server 运行时装配

- [x] T12 新建 `server/runtime.rs`：第二个组合根——构造 `Arc<dyn SessionStore>`（SessionManager）+ `Arc<dyn EventSink>`（EventBus），从 config 加载 ModelRegistry + ToolRegistry，注入 `Agent::with_ports`，暴露 `RunningServer` 句柄（graceful shutdown via signal）

## P5 — 反向 RPC

- [x] T13 `protocol.rs` 增补 `Command::ApproveTool {call_id, approved}` / `Command::AnswerQuestion {call_id, answer}`
- [x] T14 server 侧反向 RPC gateway：`HashMap<call_id, oneshot::Sender>` 注册/消费，超时 60s 推 `ApprovalTimeout`，多 client 首个应答生效

## P6 — RemoteDriver + CLI 子命令

- [x] T15 `interactive/driver.rs` 加 `RemoteDriver`：REST 发 control 命令（`send`），WS 接事件流（`subscribe`），实现 `Driver` trait
- [x] T16 CLI 增 `server run [--port]` / `server install` / `server stop` 子命令

## P7 — BDD + 集成测试

- [x] T17 更新 `rpc.feature`：适配 protocol envelope 和新增 Command 变体
- [x] T18 新建 `server.feature`：订阅→断连→工作→重连→补齐场景
- [x] T19 新建 `approval.feature`：工具执行→审批请求→用户审批→恢复场景

## 收尾

- [x] T20 全量 QA：`cargo build --all-features && cargo nextest run --profile ci && cargo clippy --all-features -- -D warnings && cargo test --test bdd -- --test-threads=1 && cargo fmt --check`
- [x] T21 `llman sdd validate c270-add-server-process --strict --no-interactive` 通过
