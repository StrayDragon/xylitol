# Tasks：c2480 Attach 连接韧性状态机

- [x] t1 wire+server 首帧 hello：`RpcMessage` 新增 `ServerHello { protocol }` 变体；server 侧 events.mux 连接建立后首帧发送；删除内部 DTO `ServerFrame::ServerHello`；单测覆盖（首帧即 hello、版本载荷 = PROTOCOL_VERSION、序列化 tag `server-hello`（信封 kebab 约定））
- [x] t2 carrier/driver 握手与重连升级 [blocked-by: t1]：`http_ws.rs` mux 流首帧必须为 `ServerHello` 否则按连接失败；`ensure_downlink` 逐连接校验版本（不符 fatal 不重试）、退役 `host.describe` 主路径与 `handshake_done` 闩存、存活 ≥ 阈值才归零退避、generation 代际失效；mock 缝可执行场景转绿（`reconnect-backoff-escalation` / `reconnect-stale-generation-dropped` / `attach-hello-mismatch-fatal`，新 steps+bindings）
- [x] t3 下行攒批合帧：短窗口（10ms 起步、可注入）内多事件合并一次投影；mock 缝可执行场景转绿（`attach-coalesce-burst-single-projection`）
- [x] t4 宽限分级 UX [blocked-by: t2]：TUI host 层初始 5s / 重连 1s 宽限（常量注入）；宽限内零打扰、超宽限壳层通告（chrome toast）断线/恢复、重连窗不刷 transcript 错误行（替换 push error_msg 路径）；TUI harness 单测断言分级与词汇
- [x] t5 真进程缝 BDD [blocked-by: t1, t2]：`serve()` 风格起真 server，场景 `real-kill-reconnect-journal-resume` 转绿（订阅 → 杀 server → 同版本重启 → last_seq 续传兑现 + 宽限内无错误行上屏）
- [x] t6 门禁收口：`just fmt` / `just lint` / `just test`（含既有 BDD + 新增场景）；`llman sdd validate --strict`
