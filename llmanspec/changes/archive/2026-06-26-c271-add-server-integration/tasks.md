# Tasks: c271-add-server-integration

> **范围**：将 c270 交付的组件焊接成可工作的 server。每阶段后全测试套件绿灯才进下一阶段。

## P1 — Agent 挂进 AppState，REST 路由功能化

- [x] T1 改 `server/runtime.rs`：`AppState` 持有 `Arc<Mutex<Agent>>`，`RunningServer` 保持句柄；在 `start()` 中将 Agent 传入 `AppState`
- [x] T2 改 `server/rest.rs`：`POST /api/v1/session/{id}/run` 提取 prompt → `agent.lock().run(prompt)` → 返回 session_id
- [x] T3 改 `server/rest.rs`：`DELETE /api/v1/session/{id}` 调用 `agent.abort()`
- [x] T4 改 `server/rest.rs`：`GET /api/v1/session/{id}/events?seq=N` 从 journal 返回事件
- [x] T5 改 `server/rest.rs`：`POST /api/v1/session/{id}/model` 切换模型
- [x] T6 移除所有 "not yet implemented" 桩，验证：build + nextest + BDD 全绿

## P2 — WS upgrade 挂载 + 事件推送

- [x] T7 改 `server/rest.rs`：挂 `GET /api/v1/session/{id}/ws` WS upgrade handler
- [x] T8 实现 WS 握手：等待 Subscribe → 发 ServerHello + Ack → 持续推送 Event 帧
- [x] T9 透传 `ReverseRpcGateway`：收到 `ApproveTool`/`AnswerQuestion` 帧时消费 gateway
- [x] T10 验证：build + nextest + BDD 全绿

## P3 — RemoteDriver 完整化

- [x] T11 改 `interactive/driver.rs::RemoteDriver::run()`：REST POST 提交 prompt + WS connect 订阅事件流（tokio-tungstenite connect）
- [x] T12 `RemoteDriver::run()` 持续推送事件直到 AgentEnd，支持有限次断连重试
- [x] T13 验证：build + nextest + BDD 全绿

## P4 — server stop 进程间信号

- [x] T14 改 `interactive/cli/mod.rs::ServerSubcommand::Stop`：读取锁文件中的 PID，发 SIGTERM（Unix），等待进程退出，再删锁文件

## P5 — 集成测试

- [x] T15 在 `tests/bdd.rs` 添加 step definitions 对应 `server.feature`（server start → healthz → run → events → stop）
- [x] T16 在 `tests/bdd.rs` 添加 step definitions 对应 `approval.feature`（tool approval round-trip via FakeProvider）

## 收尾

- [x] T17 全量 QA：`cargo build --all-features && cargo nextest run --profile ci && cargo test --test bdd -- --test-threads=1 && cargo fmt --check`
- [x] T18 `llman sdd validate c271-add-server-integration --strict --no-interactive` 通过
