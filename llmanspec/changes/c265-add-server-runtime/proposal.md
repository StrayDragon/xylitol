---
depends_on: [c260-refactor-domain-architecture]
---

# c265-add-server-runtime

> **状态**：占位提案（2026-06-26）。从 `c260-refactor-domain-architecture` 拆分而来——c260 已完成行为不变的架构重构（P0-P4 + protocol 抽取），本 change 承载后续 server 进程化新功能 + HC-2 port 落地。
> **前置依赖**：c260 archive 后方可 apply（c260 的 protocol SSOT + 薄 agent + infra 运行时归位是 server 托管的前提）。

## Why

c260 把 xylitol 重构为"薄 agent + infra 运行时域 + protocol SSOT + interactive 表皮"，为 server 常驻托管铺好了底座。但 server 本身是**新功能**（新进程、新依赖、新状态机），与 c260 的"行为不变重构"性质不同，按 SDD 原子性原则独立成 change。

本 change 兑现总览（c260 design §0）的部署形态 B：
- **核心常驻**：`server/` 托管 `agent` + `infra`，一直跑、一直工作。
- **交互可离线**：`interactive/*` 通过 `RemoteDriver`（WS/REST）连接，断连期间核心继续工作，重连补齐事件。
- **统一交互形态**：本地（InProcessDriver）与远程（RemoteDriver）client 代码逐字相同，经统一 `protocol`。

## What Changes

### P1 — HC-2 port 落地（c260 T26-T30 的真实需求驱动）
- `core/ports.rs` 新增 `SessionStore` / `EventSink` port（方法集由 server 托管场景 TDD-reverse 定义，**不抄 AgentSession 全量 surface**）
- `infra::session::SessionManager` impl `SessionStore`、`infra::event::EventBus` impl `EventSink` + test 双
- `agent::facade::Agent::new(ports...)` 注入 port，`run(prompt)` 去 session_id（HC-2 修正）
- `interactive/cli` 组合根注入 port

### P2 — Driver 抽象（c260 T36-T37）
- `interactive/driver.rs`：`Driver` trait（send/subscribe）+ `InProcessDriver`
- `interactive/{cli,print}` 改用 `Driver`

### P3 — server 进程化（c260 T38-T44）
- 引入 HTTP/WS 框架（axum + tokio-tungstenite，与既有 tokio 契合）
- `server/runtime.rs`（装配组合根）、`server/rest.rs`（`/api/v1` + envelope）、`server/ws.rs`（帧协议 + seq）、`server/lock.rs`（单实例锁 + port-retry）
- 重连 journal + `resync_required`
- 反向 RPC gateway（approval/question）

### P4 — 远程交互（c260 T45-T47）
- `interactive/driver.rs::RemoteDriver`（WS+REST），与 InProcessDriver 事件序列一致（同测试用例验证）
- CLI 加 `server run` / `server install` 子命令
- BDD：rpc.feature 适配 protocol 演进；新增 server 重连、反向 RPC 场景

## Capabilities

- `interactive-protocol`（modify）：envelope/error-code typing 补齐（c260 只立了 Command/Event）
- `server-runtime`（add）：常驻核心 + REST/WS + 重连 + 反向 RPC
- `interactive-client`（modify→add）：Driver 抽象 + RemoteDriver
- `agent-runtime`（modify）：HC-2 port 注入、facade 去 session_id
- `agent-session`（modify）：impl SessionStore
- `session-persistence`（modify）：SessionStore adapter
- `layer-architecture`（modify）：arch_guard 加 interactive 断言（c260 NOTE 里的待启用项）

## Impact

- **新依赖**：axum（HTTP）、tokio-tungstenite（WS）。需评估与现有依赖栈契合度。
- **新状态机**：重连 journal + seq + resync，配 property-based 测试。
- **行为新增**：server 进程、`server run/install` 子命令、远程交互能力。
- **风险**：高。port 边界 TDD-reverse + 重连状态机是两大难点。

> NOTE: 本提案目前是占位，apply 前需补全 specs/ delta（每个 capability 至少一个 add/modify op + scenario）与 design.md（port 方法集自检表、重连/反向 RPC 权衡）。
