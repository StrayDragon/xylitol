---
depends_on:
  - c30-align-model-registry
  - c35-align-settings-manager
  - c40-align-event-bus
  - c50-align-compaction
  - c60-align-resource-loader
  - c65-align-session-manager-tree
  - c70-align-agent-extensions
---

# c75-align-agent-session: 对齐 pi AgentSession 核心集成

## Why
当前 xylitol 的 `AgentSession` 实现了基础功能：模型切换、思考级别、压缩。pi 的 AgentSession 是 3143 行的中央枢纽，还需要集成：事件订阅自动持久化、prompt 处理管道（斜杠命令、技能块、模板展开）、会话操作（fork/navigateTree/newSession/switchSession）、扩展集成（wrapRegisteredTools）、消息队列（steering/followUp）。

## What Changes
- **增强** `src/agent/session.rs`：
  - `subscribe(listener)` 自动持久化（turn_end → append session）
  - `prompt(text)` 处理管道：斜杠命令 → 技能块 → 模板展开 → agent loop
  - 会话操作委托：`fork(entry_id)` → SessionManager.fork
  - `navigate_tree(target_id)` → SessionManager.navigate_tree
  - `new_session()`, `switch_session(path)` 委托
  - 消息队列集成：`steering_mode` / `follow_up_mode`
  - 扩展集成：wrap_registered_tools、emit_session_shutdown
  - 技能命令注册：`/skill:name` 自动注册
- **新增** `src/agent/session/services.rs`：`AgentSessionServices` 依赖注入容器
- **新增** `src/agent/session/runtime.rs`：运行模式无关的 AgentSession 运行时
- 更新 BDD 测试覆盖集成场景

## Capabilities
- agent-session

## Impact
- 破坏性变更：`AgentSession::new()` 需要 `AgentSessionServices` 参数
- `prompt()` 签名扩展支持技能块和模板展开
- `AgentLoop` 不再单独使用，成为 `AgentSession` 的内部组件
