# Tasks — c1015-add-app-tui-session-panel-slash

- [x] 1. Delta + design + tasks 齐；`llman sdd validate c1015-… --no-interactive`（apply 前确认 c1005 已归档；结束后再 `--strict`）
- [x] 2. Driver（± 论证后 protocol）`list_sessions`：mtime 降序；name + id；禁止 TUI infra reach
- [x] 3. `/session`：parse + dispatch GetSessionStats → scrollback 文本块
- [x] 4. `/session-resume`：SelectList 槽 + SwitchSession + transcript rebuild + 清槽
- [x] 5. SlashCommandSource；旧名 `/resume` 无效；busy 拒绝
- [x] 6. harness：dump stats / 开列表 / Enter switch / Esc 取消
- [x] 7. `just fmt` + 相关 test / clippy；tasks 全勾后 validate `--strict`
