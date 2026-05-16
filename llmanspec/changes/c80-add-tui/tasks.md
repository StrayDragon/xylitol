# c80-add-tui Tasks

- [ ] 实现 TUI app 主循环（crossterm event loop + ratatui 渲染）
- [ ] 实现 ChatComponent（流式文本显示）
- [ ] 实现 ToolOutputComponent（工具执行结果展示）
- [ ] 实现 DiffPreviewComponent（变更预览）
- [ ] 实现 ApprovalOverlay（命令审批流）
- [ ] 实现会话/模型/主题选择器
- [ ] 实现 Markdown 渲染（termimad 集成）
- [ ] 实现事件驱动 UI 更新（订阅 AgentEvent）
- [ ] 编写 TUI 组件测试（TestBackend + insta 快照）
- [ ] 编写 VT100Backend 完整渲染管线集成测试（feature = "dev-vt100"，双轨策略）
- [ ] 图片协议预留（Kitty/iTerm2/Sixel，Phase 2）
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c80-add-tui --strict --no-interactive`
