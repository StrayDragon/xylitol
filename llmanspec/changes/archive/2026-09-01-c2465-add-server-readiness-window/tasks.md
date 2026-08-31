# Tasks — c2465 Serve 启动就绪窗口

- [x] 1. `Gateway`（phase + host 单元格）与门禁 hoop：非 ready 时除 /healthz 外统一 503
  同语义码，不进 handler；healthz 按 D3 输出四态形态。
- [x] 2. `http.rs` router 改造：handlers 经单元格取 host（`host_from` 语义保持）；
  `serve(config, host)` bind 即 ready（既有调用点行为不变）；暴露 gated 测试缝。
- [x] 3. `runtime.rs::start()` bind-first 重构：bind → gated serve → 装配 → 翻 ready；
  装配失败 → failed 保持 10s 后报错退出；停机翻转 stopping。
- [x] 4. server-core.feature 落 `@req:sr-rdy1` + 3 条 `@executable`
  （starting 窗口 / ready 翻转 / failed 不可重试）并实现 bindings 转绿。
- [x] 5. 门禁：`just fmt` / `just lint` / `just test`；`llman sdd validate --strict`。
