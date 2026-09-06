# Tasks: c2590-fix-obs-session-per-generate

测试 seam：`xylitol-ai-bridge` 单测（重叠 generate 的 span / OpenCode header）；`infra-otel` 既有 CollectingReporter 头less 场景保持 otel6。不打网。不新扩 BDD step（与 otel18 同：单测覆盖并发槽）。

- [x] t1 specs：`infra-otel` 新增 otel23（重叠处理各带自己的 langfuse.session.id）；`package-ai-bridge` pab30（generate 快照，禁止重叠请求读进程槽）。otel6 书签语义不变。`llman sdd validate` 结构绿
- [ ] t2 `AiBridgeGenerateOptions` 携带观测会话快照；adapter/hooks/generation span 使用快照
- [ ] t3 单测：两路重叠 generate、不同书签，header 与 `langfuse.session.id` 不错位；进程槽被中途改写不得污染先发出的请求
- [ ] t4 `cargo test -p xylitol-ai-bridge` 相关单测 + 既有 otel CollectingReporter + `llman sdd validate c2590-fix-obs-session-per-generate --strict --no-check`
