# Design — c2465 Serve 启动就绪窗口：三态健康语义

## 现状核对（代码事实）

- `runtime.rs::start()`：`resolve_assembly`（config/trust/MCP spec/模型注册）→ `build_ports`
  → `HostState` → `serve()` 才 bind。**窗口内连接拿到 ECONNREFUSED**，与「没起过 / 端口错」不可区分。
- `serve()` bind 后立即挂全量 router（handlers 捕获 `Arc<HostState>`，经 `InjectHost` hoop 注入 Depot）。
- healthz 已有状态先例：`shutting_down` → 503 `{"status":"shutting_down"}`（sr-h1 只锁 ready 后 200）。

## 决策

- **D1 机制：门禁 hoop + Host 单元格（非 router swap）**。router 在 bind 时即全量挂载：
  新增 hoop 置于最前，读 `Gateway`（`phase: starting|ready|stopping|failed` +
  `OnceLock<Arc<HostState>>`）。phase ≠ ready 且路径非 /healthz → 统一 503（同语义码），
  不进入任何 handler（不半执行）；ready 后 hoop 直通，handlers 经单元格取 host
  （`host_from` 语义不变）。
- **D2 三态 + 停机**：starting（装配中）/ ready / stopping（优雅停机，沿用既有
  shutting_down 翻转点，body 词统一为 `stopping`）/ failed（装配失败，**不可重试**语义，
  无 retry-after）。failed 后监听器**有界保持 10s**（供探针读到原因）再退出。
- **D3 healthz 形态**：ready → 200 `{"status":"ok"}`（sr-h1 不变）；starting → 503
  `{"status":"starting","retry_after":1}` + `Retry-After: 1` 头（常量 1s，不加配置）；
  failed → 503 `{"status":"failed"}`（无 retry-after）；stopping → 503 `{"status":"stopping"}`。
  `{pid, version}` 字段**留给 c2475**（其提案已声明扩展此处）。
- **D4 start/serve 分工**：`start()` 改为 bind-first——bind → gated serve（Starting）→
  装配 → 填充单元格 + 翻 ready；装配失败翻 failed、保持 10s、报错退出。
  `serve(config, host)`（测试/自定义装配路径）host 已在：bind 即 ready，行为不变；
  另暴露 gated 测试缝（BDD 驱动 starting/failed 窗口）。
- **D5 specs landing**：并入 `server-core.feature`——新增 `@req:sr-rdy1 @human`（三态
  就绪语义）+ 3 条 `@executable`（starting 窗口 503 / ready 翻转 / failed 不可重试）。
  sr-h1（启动完成后 200）语义不变、不改动。

## 非目标（沿 proposal）

不改「显式 serve 占绑定」模型；不做健康检查鉴权（c2485）；不改 `{pid,version}`（c2475）。
