# c94-fix-race-conditions Tasks

- [ ] 重构 ApprovalHub：wrapper 创建 channel 后注册，UI 通过 tx.send 响应（消除 register/take 窗口）
- [ ] ApprovalHub：register 使用 Entry API，重复 call_id 返回错误而非覆盖
- [ ] Hook script：超时分支添加 `child.kill().await` + `child.wait().await`
- [ ] edit 工具：实现 write(temp) + fsync + rename 原子写
- [ ] write 工具：同上，temp 文件 + rename
- [ ] edit/write：写前记录 mtime，写前再校验一致性
- [ ] Session `ensure_session`：捕获 "already exists" 错误视为成功
- [ ] MCP：`list_all_tools`/`call_tool` 锁内仅 clone Arc，锁外 await
- [ ] history：添加 `flock` 文件锁保护写入
- [ ] step_counter：改为 `fetch_add(1, Ordering::Relaxed)`
- [ ] 添加竞态场景集成测试（模拟紧耦合 stream）
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c94-fix-race-conditions --strict --no-interactive`
