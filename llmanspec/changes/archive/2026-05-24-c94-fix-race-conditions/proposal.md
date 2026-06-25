---
depends_on: [c50-add-security, c40-add-hooks]
---

# c94-fix-race-conditions

## Why

竞态条件审查发现多处数据竞争和时序问题：

1. **TUI 审批注册/执行时序竞态**：`SecureApprovalToolWrapper::execute()` 在 `take()` 时，UI 主循环可能尚未 `register()` 该 call_id，导致工具被误拒
2. **Hook 超时后子进程未终止**：注释写 "killing" 但实现未调用 `child.kill()`，僵尸进程累积
3. **edit/write 非原子操作**：read → modify → write 期间文件可能被并发修改，导致丢失更新
4. **Session get-then-create TOCTOU**：并发 `run()` 可能都看到 "not found" 并都 `create`
5. **MCP Mutex 持锁期间 await**：`list_all_tools`/`call_tool` 持锁调用异步操作，串行化所有 MCP 调用
6. **history 文件无锁写入**：多 xylitol 实例可能互相覆盖历史记录
7. **step_counter 非原子 fetch_add**：load + store 间可能丢失计数
8. **ApprovalHub 重复 call_id 静默覆盖**

## What Changes

1. 重构审批机制：wrapper 主动创建 channel，UI 只负责 `tx.send(decision)`
2. Hook 超时必须 `child.kill().await` + drain
3. edit/write 使用原子写（temp + rename）+ 写前校验 mtime
4. Session `create` 捕获 "already exists" 视为成功
5. MCP 锁粒度缩小：锁内 clone handle，锁外 await
6. history 使用文件锁（`flock`）或 append-only
7. step_counter 改为 `fetch_add(1, Relaxed)`
8. ApprovalHub register 前检测重复（Entry API）

## Capabilities

- `tui-interface`: TUI 审批交互

## Impact

- 审批机制重构可能影响 TUI 审批流程的用户可见行为
- 原子写引入临时文件（相同目录），需确保不留残余
- MCP 并发能力提升但需验证正确性
