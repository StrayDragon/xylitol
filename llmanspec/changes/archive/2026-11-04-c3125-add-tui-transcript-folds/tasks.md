# Tasks — TUI 工具参数折叠层

- [x] 1. ScrollbackFold 增加 args_expanded / args_overrides / args_effective / toggle_args / clear_args_overrides。
- [x] 2. FoldTarget 增加 ToolArgs；点击参数 chip 切换；hover 命中与 fold hit 表覆盖。
- [x] 3. paint_tool_block：header 下画 `[args]` chip；收起只显示 chip，展开显示参数正文；默认 summary 收起。
- [x] 4. /fold args 与 visible_items 扩展；full 档展开参数。
- [x] 5. Specs：app-tui-transcript 工具参数折叠；app-tui-mouse-actions chip 命中。
- [x] 6. 测试：默认收起、点击展开、diff 保持既有；snapshot。
