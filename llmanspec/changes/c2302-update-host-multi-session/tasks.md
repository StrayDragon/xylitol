# Tasks

测试边界：现有 `server-core` `.feature`（改写后）+ `cargo test --test bdd` + salvo `TestClient`（healthz）+ 真 bind 的 EADDRINUSE。不新开脱离 `.feature` 的 CLI 子进程协议。

## 1. 合约

- [ ] 1.1 改 `server-core` spec.toon + `server-runtime.feature` / `server-ws.feature`：绑定占用、无产品 REST、单 `/ws`、写者
- [ ] 1.2 改 `layer-architecture` la6：监听器非整机锁；产品面是 WS 不是 REST

## 2. salvo 监听器

- [ ] 2.1 `server` feature：axum/tower-http → salvo 0.94 `websocket`
- [ ] 2.2 bind `127.0.0.1:18790`；EADDRINUSE 失败；删锁文件与 port+1
- [ ] 2.3 `GET /healthz` + `ServerHandle::stop_graceful`；`server stop` 不再依赖锁文件

## 3. WS 产品面

- [ ] 3.1 `/ws` 升级：`check_origin` 允许缺 Origin；bounded per-client mpsc；tagged JSON 帧
- [ ] 3.2 每 session journal / seq / reverse RPC 迁到 salvo 回调；多 session 注册表
- [ ] 3.3 一写者；只读连接写操作回 Error
- [ ] 3.4 RemoteDriver 改 WS；删除产品 REST 路由（含 session tree REST）

## 4. 闸

- [ ] 4.1 `llman sdd validate c2302-update-host-multi-session --strict`
- [ ] 4.2 `cargo test --test bdd` 与 `just qa` 绿
