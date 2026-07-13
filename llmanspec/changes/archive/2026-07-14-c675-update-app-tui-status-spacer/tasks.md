# Tasks — c675-update-app-tui-status-spacer

- [x] 1. Delta 校验：`LLMANSPEC_BASE_REF=origin/main llman sdd validate c675-update-app-tui-status-spacer --no-interactive`
- [x] 2. `render_status_slot`：busy 保留 Loader 前导空行；idle 一行 blank
- [x] 3. harness：busy blank+spinner；idle blank；不破坏既有 abort/busy 断言
- [x] 4. `just fmt` + 相关 test / clippy 绿
