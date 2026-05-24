# c94-fix-race-conditions Tasks

- [ ] 重构 ApprovalHub：wrapper 创建 channel 后注册，UI 通过 tx.send 响应（defer - 需要较大重构）
- [ ] ApprovalHub：register 使用 Entry API（defer - 与上项联动）
- [x] Hook script：超时改为 fail-closed（已在 c91 实现）
- [x] edit 工具：实现 atomic write（temp + rename）
- [x] write 工具：实现 atomic write（temp + rename）
- [ ] edit/write：写前 mtime 校验（defer - 需要额外 IO 与性能权衡）
- [ ] Session `ensure_session`：捕获 "already exists"（defer - 需了解 adk-session API）
- [ ] MCP：锁粒度优化（defer - 需验证 Arc clone 语义）
- [ ] history：文件锁（defer - 多实例场景低优先级）
- [x] step_counter：改为 `fetch_add(1, Ordering::Relaxed)`
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c94-fix-race-conditions --strict --no-interactive`
