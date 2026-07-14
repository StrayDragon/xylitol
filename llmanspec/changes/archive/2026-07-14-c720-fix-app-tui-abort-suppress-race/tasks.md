# Tasks — c720-fix-app-tui-abort-suppress-race

- [x] 1. 确认 c715 已 archive；`LLMANSPEC_BASE_REF=origin/main llman sdd validate c720-fix-app-tui-abort-suppress-race --no-interactive`
- [x] 2. `try_busy_input` Esc：同步臂装 `suppress_xy_until_stream_end`；保留 `pending_abort`
- [x] 3. `note_user_abort` 幂等（重复臂装 / 已 Aborted 不双写）
- [x] 4. harness：Esc → 注入 TextDelta/AgentEnd → drain → 无正文复活；BDD abort 场景对齐
- [x] 5. 跑 c715 BASE/ABS + 相关 bdd；`just fmt` + clippy
- [x] 6. `validate --strict`；准备 archive
