---
id: c180-rebuild-agent-session
title: "Rebuild AgentSession as event-driven state machine — align with pi's AgentSession"
depends_on: [c170-refactor-agent-message-types, c175-refactor-event-system]
---

## Why

当前 AgentSession 是一个结构体容器——拥有所有组件（模型注册表、工具、队列、压缩设置等），但没有任何编排逻辑或事件驱动行为。与 pi 的 AgentSession 相比，当前实现在以下方面严重缺失：

1. **无事件驱动**：没有 subscribe/emit 模式，事件发不出也收不到
2. **无消息队列消费**：steering/followUp 队列定义了但从不被消费
3. **无自动重试**：每次代理运行后不检查是否需要重试
4. **无自动压缩**：Agent 完成后不检查 token 阈值
5. **无 Bash 刷新**：Bash 输出待处理但从不刷新到上下文
6. **无扩展命令**：不支持 `/cmd` 扩展命令、`/skill:name`、`/template:name`
7. **无 dispose/abort 生命周期**：资源无法正常释放

## What Changes

重写 AgentSession 为事件驱动的状态机，包含：

1. **订阅系统**：`subscribe(AgentSessionEventListener)` + 内部 persistence handler
2. **消息队列编排**：`prompt()` → 处理 `/cmd`/`/skill`/`/template` → 构建消息 → `_runAgentPrompt()`
3. **steering/followUp**：`steer()` / `followUp()` 方法和 AgentLoop 协作的双层循环
4. **自动重试**：`_willRetryAfterAgentEnd()` + `_prepareRetry()` + `_isRetryableError()`
5. **自动压缩**：`_checkCompaction()` 在每个 assistantMessage.agent_end 检查
6. **Bash 执行集成**：`executeBash()` + 输出收集 + 暂停刷新
7. **扩展命令 + 技能 + 模板展开**：统一的 `_expandSkillCommand()` / `expandPromptTemplate()`
8. **生命周期管理**：`dispose()` 清理所有资源；`abort()` 取消操作
9. **模型/思维级别管理**：带事件发射和认证检查的 setModel/cycleModel/setThinkingLevel
10. **设置管理器集成**：从 session.settingsManager 获取压缩/重试/队列模式设置
11. **clearQueue() / pendingMessageCount()** 等实用方法

## Capabilities

- agent-session

## Impact

- `src/agent/session.rs`：完全重写
- `src/agent/loop.rs`：与 session 的 queue/retry/compaction 交互
- `src/agent/queue.rs`：保留但集成到 session 方法中
- `src/agent/retry.rs`：保留但 session 驱动
- `src/agent/commands.rs`：保留但 session 提供 dispatch
- `src/infra/settings/`：集成到 AgentSession 读取设置
- 删除 `src/agent/event.rs`（由 c175 替代）

## Definition of Done

- [ ] AgentSession 实现了 subscribe/emit/UnsubscribeHandle
- [ ] prompt() 支持 `/cmd` 分发、`/skill:name` 展开、`/template:name` 展开
- [ ] 支持 steering/followUp 消息队列，与 Loop 协作
- [ ] 自动重试：agent_end 后检查并发射 auto_retry_start/end 事件
- [ ] 自动压缩：agent_end 后检查 token 阈值并触发 compact
- [ ] `dispose()` 清理所有控制器（重试/压缩/bash）
- [ ] `abort()` 取消当前操作并等待 idle
- [ ] `setModel()` / `cycleModel()` / `setThinkingLevel()` 工作并持续化
- [ ] `cargo test` 全部通过
