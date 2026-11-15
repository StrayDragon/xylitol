# Tasks — 空输入提示行与快捷键面板

- [ ] 1. 新增 `help_open` / `help_scroll` 状态；实现 hint 行渲染；更新 `last_dock_rows`、`editor_top_row` 与 Fixed-Zone Footprint。约 1.5h。
- [ ] 2. 接入 `?` 键、Esc/Ctrl+C 优先级、面板滚动键与 busy 优先级。约 1h。
- [ ] 3. 实现快捷键面板：从 keybindings 取键位，分组、中文标签、3/2/1 列、滚动。约 2h。
- [ ] 4. 增加 harness 测试：空输入 hint、输入后隐藏、`?` 开关、非空 `?` 字面、Esc/Ctrl+C 优先级、busy、动态键位、窄终端。约 2h。
- [ ] 5. 更新 specs（`app-tui-fixed-zone` r3356/r1228；`app-tui-input` r3357）并跑 `llman-sdd validate`。约 1h。
- [ ] 6. 跑 `just fmt`、相关 TUI 测试与 `just qa`。约 1h。
