# c90-update-markdown-rendering Tasks

- [ ] 盘点当前 Print/TUI 的 Markdown 渲染路径与依赖（解析/高亮/主题/换行）
- [ ] 增加 Markdown 渲染快照测试基准（覆盖宽字符/长行/代码块/链接/表格等）
- [ ] 调研候选渲染方案并记录对比（正确性/性能/许可/维护状态）
- [ ] 选定并集成默认方案（保持 raw output 兜底）
- [ ] 补齐配置项（主题/链接样式/渲染开关）并更新文档
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c90-update-markdown-rendering --strict --no-interactive`
