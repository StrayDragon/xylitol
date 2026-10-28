# Tasks

- [x] 1. **specs**（spec）：`server-core` 增 `r2973` 会话切换与订阅的模式事件路由规则；`app-tui-fixed-zone` 的 `r2956` 允许端侧在切换/恢复时做一次初始快照（事件到达后以事件为准）。测：`llman-sdd validate --all --strict`。
- [x] 2. **host**（serve）：`switch_session` / `new_session` dispatch 后按新会话 id 重新取槽并发布 `planModeChanged`，不再落到切换前的槽；`bind_mux_to_session` 在绑定后向订阅者补发一次由最新 `plan_mode` 推导的 `planModeChanged`（不写 journal、不耗 seq）。测：`cargo test -p xylitol-serve occupancy::host::tests`。
- [x] 3. **tests**（serve）：两个槽 + `publish_plan_mode_event` 断言事件只落新槽；订阅绑定断言收到快照且 journal seq 不变。测：`cargo test -p xylitol-serve`。
- [x] 4. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
