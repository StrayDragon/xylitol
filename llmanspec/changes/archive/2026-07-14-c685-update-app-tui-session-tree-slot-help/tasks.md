# Tasks — c685-update-app-tui-session-tree-slot-help

- [x] 1. Delta 校验：`LLMANSPEC_BASE_REF=origin/main llman sdd validate c685-update-app-tui-session-tree-slot-help --strict --no-interactive`
- [x] 2. Search 行：空查询 / 有查询两种渲染接入 Tree 槽
- [x] 3. TreeHelp：从 KeybindingsManager（或封装）生成 move/page/branch/filters/cycle 行；替换硬编码 hint
- [x] 4. FilterMode::cycle_backward + 树开 Ctrl+Shift+O（或解析和弦）优先路由
- [x] 5. harness：树槽 Search/Help 含动态片段；cycleBackward；Search 行随查询变
- [x] 6. 同步 `session-tree.md` / `session-tree-vs-pi.md` / `keybindings.md`（文案避开「chrome」）
- [x] 7. `just fmt` + 相关 test / clippy 绿
