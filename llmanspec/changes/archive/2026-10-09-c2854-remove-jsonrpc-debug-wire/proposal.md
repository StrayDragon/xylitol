---
depends_on: []
branch: sdd/c2854-remove-jsonrpc-debug-wire
base_branch: main
base_sha: 02ebd74ced9be14b5b40408fcb9adbd533c30a4d
---

# 退役 JSON-RPC 文本调试通道，产品 /rpc 只走 fory

## Why

产品 TUI / CLI attach 已经只说 v3 fory。线上仍双读双写 JSON 文本 `/rpc`，并挂着 `/openapi.json` 与 Scalar `/docs`。那套文档描述的是 JSON 信封，不是 fory 真源，维护税在双轨客户端、双轨 BDD 和 `salvo-oapi`。公开文本通道没有产品消费者，应退役。

## What Changes

- `POST /rpc` 与 `WS /rpc` 只接受 v3 二进制帧。JSON 文本 body / WS 文本业务帧 MUST 以非法信封拒绝（HTTP 400 同梯队）。MUST NOT 再提供 JSON-RPC 2.0 文本调试通道。
- 删除 `GET /openapi.json` 与 `GET /docs`。MUST NOT 再暴露 OpenAPI / Scalar。
- 保留同一端口 `GET /healthz`（pid / version、starting→ready 窗口）。不另起监听器或进程。
- `host.describe` 的 formats 只宣告 `fory-v3`。产品 attach 客户端只说 v3，MUST NOT 默认 JSON、MUST NOT 静默降级。
- 内部 `dispatch_raw` 的 JSON 形 MAY 保留为实现方言，MUST NOT 再作为产品可观察载体。
- 原双轨对拍场景改成 **fory-only 回归**（事件流、快照、session_tree、深树），不删行为覆盖；「JSON 仍可服务」类场景改为「JSON 被拒」。
- HTTP 宿主仍用现有框架；关掉未使用的默认 feature，去掉 OpenAPI 依赖。本 change **不**换 HTTP 框架。
- 无磁盘迁移：Host 与 TUI 同步发版；旧 JSON 客户端失败闭合。`PROTOCOL_VERSION` 保持 3。
- apply 收尾写入仓库 skill `xylitol-dev-candidates`：salvo / jsonrpsee / axum / Scalar 用过、为何拿掉、何时再考虑。

## 非目标

- 不把 fory 解码后的内部 JSON 分发改成纯二进制内核。
- 不另起标准库 HTTP 进程承载 healthz。
- 不把 salvo 换成 hyper / axum（记入 skill，另开 change）。
- 不删租约 / 幂等 / 审批等产品 BDD，只把它们改跑 fory。

## Capabilities

- `server-core`（入口、healthz、OpenAPI、双轨、formats、WS 纪律）
- `protocol-app`（单一载体、信封、非法信封）
- `layer-architecture`（server 应用面载体措辞）
- `app-tui-bridge`（默认 attach 载体）

## Impact

- 代码：`src/app/server/http.rs`、`oapi.rs`（删）、`wire_v3.rs`、`rpc_module.rs` 调用点、`host_client/http_ws.rs`、`driver/remote.rs`；`Cargo.toml` 去 `salvo-oapi` 并瘦 salvo feature。
- 测试：`tests/bdd/steps_server.rs` / `steps_wire_v3.rs` / 绑定；lib 双轨对拍改 fory-only。
- 文档：`docs/architecture/远程体验与线协议.md`、相关 AGENTS 注释。
- 破坏性：curl JSON `/rpc`、旧 JSON attach、浏览器 `/docs` 全部失效。

## Further Notes

- 对拍核查：方法表 41 项与 registry 对齐单测锁定；fory 上行转同一 `dispatch_raw`；产品面 `XyRemoteDriver::new_v3`；`dual_rail_event_stream_parity` 与 `dual_rail_get_messages_equivalence` 本机绿。多数 unary 应答仍是 `RawOk`（二进制信封里的 JSON 原文）——领域等价，不是每方法独立 fory schema。
- 原 r1908「两条路径对拍」收口为「fory 路径覆盖原对拍所锁行为」；不再要求 JSON 孪生。
