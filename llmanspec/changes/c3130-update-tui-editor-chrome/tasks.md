# Tasks — TUI 编辑区 chrome 重设计

- [ ] 1. 上/下边框改圆角 `╭╮╰╯`；标题右对齐上边框；不增高。
- [ ] 2. 移除 `[详情]` chip 与默认详情入口；保留既有 popup 代码但不再由 chip 打开（后续清理）。
- [ ] 3. chip 分栏：上边框 `[侧栏]`；下边框右端 `[▤ 摘要]`；重置 chip 仅覆盖时显示，位于摘要左侧。
- [ ] 4. `◨` 输入内容行右端；固定 `⬢` spinner（颜色脉冲后续）。
- [ ] 5. 点击命中随绘制行更新；窄宽整 chip 不画。
- [ ] 6. Specs：app-tui-fixed-zone 编辑区 chrome；app-tui-mouse-actions chip 命中。
- [ ] 7. 测试：80/120/160 snapshot；chip 点击；无详情 chip。
