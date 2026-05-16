# c75-add-diff-review Tasks

- [ ] 定义共享数据结构（ReviewComment, ReviewVerdict, CommentSeverity）
- [ ] 实现 CLI 终端内评审（ratatui diff 渲染 + 语法高亮）
- [ ] 实现行级评论编辑器（ratatui-textarea）
- [ ] 实现交互键位（j/k/c/a/r/e）
- [ ] 实现 Web 浏览器评审后端（axum HTTP server + Monaco Editor CDN）
- [ ] 实现审查流程集成（步骤完成 → diff 生成 → 评审 → Accept/Reject）
- [ ] 编写测试（评论数据结构、diff 渲染、审查流程）
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c75-add-diff-review --strict --no-interactive`
