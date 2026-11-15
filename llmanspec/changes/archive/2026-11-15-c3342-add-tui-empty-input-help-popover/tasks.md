# Tasks — 空输入提示行与快捷键面板

- [x] 1. 新增 `help_open` / `help_scroll` 状态；实现 hint 行渲染；更新 `last_dock_rows`、`editor_top_row` 与 Fixed-Zone Footprint。约 1.5h。
- [x] 2. 接入 `?` 键、Esc/Ctrl+C 优先级、面板滚动键与 busy 优先级。约 1h。
- [x] 3. 实现快捷键面板：从 keybindings 取键位，分组、中文标签、3/2/1 列、滚动。约 2h。
- [x] 4. 增加 harness 测试：空输入 hint、输入后隐藏、`?` 开关、非空 `?` 字面、Esc/Ctrl+C 优先级、busy、动态键位、窄终端。约 2h。
- [x] 5. 更新 specs（`app-tui-fixed-zone` r3356/r1228；`app-tui-input` r3357）并跑 `llman-sdd validate`。约 1h。
- [x] 6. 跑 `just fmt`、相关 TUI 测试与 `just qa`。约 1h。`just qa` 已验证通过（3892 tests passed；WASI SDK 用本地安装）。
- [x] 7. 提取 `xylitol-tui::BorderedPanel`（标题/边框/内容高度/滚动）并加组件测试。约 1.5h。已验证：`cargo test -p xylitol-tui --lib` 395 passed。
- [x] 8. 将产品 help_panel 改为 `HelpPanel`（open/scroll 状态、内容、键位）并接线 `BorderedPanel`；更新 harness。约 1.5h。已验证：`cargo test -p xylitol-app-tui --lib` 823 passed。
- [x] 9. 跑 `just fmt`、`cargo test -p xylitol-tui --lib`、`cargo test -p xylitol-app-tui --lib`、`just qa-e2e`、`llman-sdd validate --check --strict`、`llman-sdd review`。约 1h。已验证：`just qa-e2e` 3896 tests + 32 PTY passed。
- [x] 10. `BorderedPanel` 可选 border/title paint 注入并加测试。已验证：`cargo test -p xylitol-tui --lib` 396 passed。
- [x] 11. 统一 key_format（help_panel/session_tree）并加测试。已验证：`cargo test -p xylitol-app-tui --lib` 826 passed。
- [x] 12. `HelpPanel` 写入前 clamp scroll；面板高度按可用行 cap；跑 `just qa-e2e`。已验证：3896 tests + 32 PTY passed。
