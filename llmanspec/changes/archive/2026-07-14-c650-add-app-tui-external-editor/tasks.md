# Tasks — c650-add-app-tui-external-editor

- [x] 1. Delta 校验：`llman sdd validate c650-add-app-tui-external-editor --no-interactive`（工件 full；`--strict` 待其余任务勾完）
- [x] 2. 同步 DESIGN：`bash-mode.md` / `keybindings.md`（产品严格无 nano 默认；失败 → `UiEntry::Error`；与 demo 分叉写明）
- [x] 3. 产品侧独立实现 resolve/tempfile/spawn（`src/app/tui/`，**不**进包、**不**抽 demo 共享）；仅调用 `with_terminal_suspended`
- [x] 4. `Host::try_ctrl_g`：TTY+已配置 → 真路径；harness/非 TTY → stub；未配置/失败 → `UiEntry::Error` + 保留原文
- [x] 5. harness：既有 stub 回归；缺 VISUAL/EDITOR（或注入失败）→ Error 可观测且不崩；**不** spawn 真编辑器
- [x] 6. `just fmt` + 相关 test / clippy 绿
