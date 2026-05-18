# c75-add-diff-review Tasks

- [x] 定义共享数据结构（ReviewComment, ReviewVerdict, CommentSeverity）
- [x] 实现 CLI 终端内评审（ratatui diff 渲染 + 语法高亮）
- [x] 实现行级评论编辑器（ratatui-textarea）
- [x] 实现交互键位（j/k/c/a/r/e）
- [x] 实现 Web 浏览器评审后端（axum HTTP server + Monaco Editor CDN）
- [x] 实现审查流程集成（步骤完成 → diff 生成 → 评审 → Accept/Reject）
- [x] 编写测试（评论数据结构、diff 渲染、审查流程）
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c75-add-diff-review --strict --no-interactive`
