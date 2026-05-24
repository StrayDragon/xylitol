# c101-integrate-leaf-markdown-renderer Tasks

- [ ] 添加 `leaf-core = { path = "../leaf/crates/leaf-core" }` 到 Cargo.toml
- [ ] 评估 ratatui 版本差异（0.29 vs 0.30）：决定统一版本还是在边界做类型转换
- [ ] 重构 `MarkdownRenderer::render` 委托给 `leaf_core::MarkdownRenderer`
- [ ] 保留 `set_link_styled` / `set_theme` 接口，映射到 leaf-core 主题系统
- [ ] 移除自研渲染循环（`push_line`、`start_line`、`current_style` 等辅助函数）
- [ ] 更新 `markdown-rendering` spec：新增 leaf 相关需求和场景
- [ ] 更新 insta 快照测试（`cargo insta test --review`）
- [ ] 验证 Print 模式和 TUI 模式下的渲染一致性
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c101-integrate-leaf-markdown-renderer --strict --no-interactive`
