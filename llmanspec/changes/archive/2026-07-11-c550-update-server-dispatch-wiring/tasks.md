# Tasks — c550-update-server-dispatch-wiring

- [x] 1. 盘点 REST handlers → Command 映射表（对照 design）
- [x] 2. 改写可映射 handlers 走 dispatch；保持 Envelope 形状
- [x] 3. 更新 architecture 文档缺口表；RemoteDriver 回归抽测
- [x] 4. `llman sdd validate c550-update-server-dispatch-wiring --strict --no-interactive`
- [x] 5. `just lint` + server/rest 测试
