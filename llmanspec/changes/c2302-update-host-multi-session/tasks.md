# Tasks

测试边界：现有 `server-core` `.feature`（改写后）+ `cargo test --test bdd` + salvo `TestClient`（healthz）+ 真 bind 的 EADDRINUSE。不新开脱离 `.feature` 的 CLI 子进程协议。

**禁止** attach 已有 `sdd/c2302-update-host-multi-session`（旧 WS-only Command/Event landing）。须在 c2290 归档或本分支已含其合约后再 `change start`。

## 1. 合约

- [x] 1.1 改 `server-core` spec.toon + `server-runtime.feature` / `server-ws.feature`：绑定占用、无产品 REST、POST unary、WS **只下行**、写者
- [x] 1.2 改 `layer-architecture` la6：监听器非整机锁；产品面是四象限 HTTP+WS 下行，不是 REST，也不是全双工 WS Command

## 2. salvo 监听器

- [x] 2.1 `server` feature：axum/tower-http → salvo 0.94 `websocket`
- [x] 2.2 bind `127.0.0.1:18790`；EADDRINUSE 失败；删锁文件与 port+1
- [x] 2.3 `GET /healthz` + `ServerHandle::stop_graceful`；`server stop` 不再依赖锁文件

## 3. 四象限载体

- [x] 3.1 `POST /api/{method}` 只展开 c2290 方法表 unary；`POST /api/respond`；HTTP 200 + `RpcResult`；非法信封 400；未登记 method 失败
- [x] 3.2 `GET /api/events.mux` 升级：`check_origin` 允许缺 Origin；bounded per-client mpsc；只推方法表下行 `ServerRequest`（`session/event` 等）
- [x] 3.3 每 session 槽：独立 journal / seq / 写者；首次非只读 unary 才 lazy 物化隔离 Driver（共享 `RuntimePorts` 基线）。只读连接不物化写者。reverse RPC 挂在槽上，应答走 `POST /api/respond`
- [x] 3.4 一写者；只读连接写操作回业务 error
- [x] 3.5 接上 c2290 `HttpWsClient`；删除产品 REST 路由与全双工 `ClientFrame`
- [x] 3.6 把 `sr-env1` 从「客户端实现了 HostClient」升级为真 bind + POST unary + WS 下行往返；审批走 `POST /api/respond`；`subscribe` 进 `run()` / journal 重连

## 4. 闸

- [ ] 4.1 `llman sdd validate c2302-update-host-multi-session --strict`
- [ ] 4.2 `cargo test --test bdd` 与 `just qa` 绿
