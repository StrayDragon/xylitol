# Tasks — c1020-add-app-tui-session-lifecycle-slash

- [x] 1. Delta + design + tasks 齐；`llman sdd validate c1020-add-app-tui-session-lifecycle-slash --no-interactive`
- [x] 2. Driver + store 默认：`new_session` / `get_session_name` / `set_session_name`（InProcess + Scripted + Stub/Remote stub）
- [x] 3. `PendingSlash` + `parse_slash_command`：new / clone / name；旧名无效
- [x] 4. effects：new → 清 transcript；clone → fork At + switch；name 显示/设置；busy 拒绝
- [x] 5. SlashCommandSource + unknown 提示串；`PI_DELTAS` A07
- [x] 6. harness：h33 new；h34 clone At / no-leaf；h35 name set+show；旧名 unknown
- [x] 7. `just fmt` + 相关 test / clippy；tasks 全勾后 `llman sdd validate … --strict --no-interactive`
