# c90-update-markdown-rendering Tasks

- [x] 盘点当前 Print/TUI 的 Markdown 渲染路径与依赖（pulldown-cmark + syntect）
- [x] 增加 Markdown 渲染快照测试基准（覆盖宽字符/代码块/链接/表格/列表/引用等 11 项）
- [ ] 调研候选渲染方案并记录对比（defer - 当前实现已满足需求）
- [ ] 选定并集成默认方案（defer - 保持当前 pulldown-cmark+syntect 方案）
- [ ] 补齐配置项（defer - 主题切换 API 已存在，需求不急迫）
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c90-update-markdown-rendering --strict --no-interactive`
