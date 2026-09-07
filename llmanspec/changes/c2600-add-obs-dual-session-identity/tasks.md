# Tasks: c2600-add-obs-dual-session-identity

测试 seam：infra-otel CollectingReporter + session header 单测 + ai-bridge 观测属性单测。不打网。不新扩 BDD（与 otel6 headless 并存，双 id / 树边由单测钉）。

- [ ] t1 specs：`infra-otel` 新增 otel26（`xylitol.session.id` + `llm_gateway_session_id` + 树边；`langfuse.session.id` 仍等于当前 session 且 MUST 同时写 `xylitol.session.id`）；`agent-session-store` 钉 fork 写入 `header.forkAtEntryId`。不改已锁 otel6 句面
- [ ] t2 fork 写 `parentSession` + `forkAtEntryId`；观测快照/span 写出四键；无父省略树边。c2620 未落地时 `llm_gateway_session_id` MAY 等于 `xylitol.session.id` 但键 MUST 在
- [ ] t3 单测：新会话无树边键；fork 子本 header 与 span 树边一致；`langfuse.session.id` == `xylitol.session.id`；旧文件无 `forkAtEntryId` 时省略切点属性
- [ ] t4 相关单测 + `llman sdd validate c2600-add-obs-dual-session-identity --strict --no-check`
