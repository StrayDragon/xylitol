# Tasks: Phase 2 Core Gaps

## 阶段 1: 会话树形结构（基础依赖，最先实现）

### 1.1 Entry 类型升级
- [x] T1: `src/infra/session/types.rs` — `EntryBase` 增加 `id: String` 和 `parent_id: Option<String>` 字段；所有 entry 变体包含 EntryBase
- [x] T2: `src/infra/session/types.rs` — 添加 `ModelChangeEntry`, `ThinkingLevelChangeEntry`, `CustomMessageEntry` 变体到 SessionEntry 枚举
- [x] T3: `src/infra/session/types.rs` — 定义 `SessionContext { messages: Vec<XyContent>, thinking_level, model }` 结构体

### 1.2 SessionManager CRUD 增强
- [x] T4: `src/infra/session/manager.rs` — `append()` 自动生成 UUID id + 继承 parent_id（从当前 leaf_id）
- [x] T5: `src/infra/session/manager.rs` — `load()` 支持 v3→v4 迁移（为无 id 的旧条目注入 id/parent_id）
- [x] T6: `src/infra/session/manager.rs` — `get_entry(id)` 按 id 查找条目
- [x] T7: `src/infra/session/manager.rs` — `leaf_id/leaf_entry/branch(entry_id)/reset_leaf()` 树导航
- [x] T8: `src/infra/session/manager.rs` — `get_branch(leaf_id)` 从 leaf 到 root 收集条目

### 1.3 buildSessionContext
- [x] T9: `src/infra/session/manager.rs` — `build_session_context(session_id)` — 沿树从 leaf→root 重建会话消息
- [x] T10: 单元测试: 树导航 + buildSessionContext + v3 迁移

### 1.4 Change Tracking
- [x] T11: `src/infra/session/manager.rs` — `append_model_change(provider, model_id)` 写入 ModelChangeEntry
- [x] T12: `src/infra/session/manager.rs` — `append_thinking_level_change(level)` 写入 ThinkingLevelChangeEntry
- [x] T13: 单元测试: model_change + thinking_level_change 持久化

**检查点**: `cargo test` — 所有 session 测试通过，树形结构可用 ✅

---

## 阶段 2: 事件订阅模型重构

### 2.1 事件总线
- [x] T14: `src/agent/event.rs` — 创建 `AgentEventBus`（`tokio::sync::broadcast` 多订阅者模式）
- [x] T15: `src/agent/event.rs` — `subscribe() -> UnsubscribeHandle`，drop handle 自动取消订阅
- [x] T16: `src/agent/event.rs` — `emit(event)` 发送到所有活跃订阅者

### 2.2 AgentLoop 集成
- [x] T17: `src/agent/loop.rs` — AgentEventStream::fan_out(bus) 桥接到 EventBus
- [x] T18: `src/agent/loop.rs` — 保持向后兼容：现有 Consumer 仍可通过 Stream 消费事件

### 2.3 多订阅者测试
- [x] T19: 测试: 两个消费者同时订阅，都收到完整事件流
- [x] T20: 测试: 取消订阅后不再收到事件

**检查点**: `cargo test` — 所有现有测试适配新事件模型 ✅

---

## 阶段 3: Agent 循环增强（compaction + retry + queue）

### 3.1 队列管理
- [x] T21: `src/agent/queue.rs` — `MessageQueue` 结构体：steer 队列 + followUp 队列
- [x] T22: `src/agent/queue.rs` — `steer(text)`, `follow_up(text)`, `clear()`
- [x] T23: `src/agent/queue.rs` — `get_steering()`, `get_followup()`（不可变只读）
- [ ] T24: `src/agent/loop.rs` — 集成队列：在 tool execution 完成后、下次 LLM 调用前 drain steer 队列；在 AgentEnd 前 drain followUp

### 3.2 自动重试
- [x] T25: `src/agent/retry.rs` — `is_retryable_error(error_msg) -> bool`
- [x] T26: `src/agent/retry.rs` — `RetryState` 结构体：max_retries, base_delay, attempt, abort
- [ ] T27: `src/agent/loop.rs` — 在 assistant error 后检查 retryable，移除错误消息，指数退避等待，重试
- [ ] T28: `src/agent/loop.rs` — 重试事件：retry_start/retry_end 发送到 EventBus

### 3.3 自动压缩集成
- [ ] T29: `src/agent/loop.rs` — 在 agent_end 事件后检查 compaction 条件（threshold / overflow）
- [ ] T30: `src/agent/loop.rs` — overflow recovery 逻辑：检测 context overflow 错误 → 移除 error msg → compact → retry once
- [ ] T31: `src/agent/loop.rs` — threshold 自动压缩：估计 tokens + should_compact → auto-compact → 继续
- [ ] T32: 压缩/重试/溢出恢复全部 abortable

### 3.4 测试
- [ ] T33: BDD: steer/followUp 队列场景
- [ ] T34: BDD: auto-retry 场景（retryable + non-retryable errors）
- [ ] T35: BDD: auto-compaction 场景（threshold + overflow）

**检查点**: `cargo test --test bdd` — 新增场景全部通过

---

## 阶段 4: AgentSession 增强

### 4.1 系统提示词动态构建
- [ ] T36: `src/agent/prompt.rs` — `build_system_prompt(opts)` 函数：拼接工具片段 + 指南 + skills + context_files + 日期 + CWD
- [ ] T37: `src/agent/session.rs` — `set_active_tools(names)` 时重建系统提示词
- [ ] T38: ToolRegistry 增加 `prompt_snippet()` 接口（每个工具返回一行短描述）

### 4.2 会话统计
- [ ] T39: `src/agent/session.rs` — `get_session_stats()` 实现（调用 SessionManager + 计算 tokens/cost）
- [ ] T40: `src/infra/session/manager.rs` — `list_with_metadata()` 返回 `Vec<SessionInfo>`（id, cwd, name, created, modified, message_count, first_message）

### 4.3 Change Tracking 集成
- [ ] T41: `src/agent/session.rs` — `set_model()` 调用 `append_model_change()`
- [ ] T42: `src/agent/session.rs` — `set_thinking_level()` 调用 `append_thinking_level_change()`

### 4.4 Custom Message + Next-turn support
- [ ] T43: `src/agent/session.rs` — `send_custom_message(custom_type, content, display)` → 持久化 CustomMessageEntry
- [ ] T44: `src/agent/session.rs` — `send_message_for_next_turn(message)` → 注入到下次 prompt 前

### 4.5 测试
- [ ] T45: BDD: 系统提示词动态构建场景
- [ ] T46: BDD: session stats + metadata 场景
- [ ] T47: BDD: change tracking 集成场景

**检查点**: `cargo test --test bdd` — 所有 agent-session 场景通过

---

## 阶段 5: LLM 分支摘要升级

### 5.1 结构化摘要提示词
- [ ] T48: `src/infra/session/compaction.rs` — 添加 `BRANCH_SUMMARY_PROMPT` 常量（Goal/Progress/Next Steps 格式）
- [ ] T49: `src/infra/session/compaction.rs` — `generate_branch_summary_llm(model, entries, reserve_tokens) -> String`

### 5.2 Token Budget
- [ ] T50: `src/infra/session/compaction.rs` — `prepare_branch_entries(entries, token_budget)`：从最新到最旧添加消息直到预算耗尽
- [ ] T51: `src/infra/session/compaction.rs` — 文件操作抽取 + `<read_files>`/`<modified_files>` XML 追加

### 5.3 迭代更新
- [ ] T52: `src/infra/session/compaction.rs` — 检测已存在的 compaction/branch_summary 条目，使用 UPDATE_SUMMARIZATION_PROMPT 迭代更新

### 5.4 集成
- [ ] T53: `src/infra/session/manager.rs` — `fork()` 使用 LLM summary（当 model 可用时，否则 fallback 文本）
- [ ] T54: `src/infra/session/manager.rs` — `branch_with_summary()` 方法：移动 leaf + 生成分支摘要

### 5.5 测试
- [ ] T55: BDD: LLM branch_summary 结构化输出场景
- [ ] T56: BDD: fallback 纯文本场景
- [ ] T57: BDD: iterative update 场景

**检查点**: `cargo test --test bdd` — 所有 compaction 场景通过

---

## 阶段 6: 文档与 QA

- [ ] T58: BDD feature 文件更新：新增场景到 tests/features/session.feature, agent.feature, compaction.feature
- [ ] T59: `just qa` 全绿（fmt + clippy + test + doc + prek）
- [ ] T60: `llman sdd validate c20-phase2-core-gaps --strict --no-interactive` 通过

---

## 验收标准

- [ ] 250+ 测试全部通过（包括新增 BDD 场景，目标 300+）
- [ ] 会话树形结构：id/parentId, branch/leaf, buildSessionContext, v3→v4 迁移
- [ ] 事件订阅：多监听器, unsubscribe, 向后兼容 Stream 适配器
- [ ] 自动压缩：threshold + overflow recovery, abortable
- [ ] 自动重试：pattern matching + exponential backoff, abortable
- [ ] 队列管理：steer/followUp, 只读访问, clear
- [ ] 系统提示词：动态构建, 工具变更时重建
- [ ] 会话统计：getSessionStats + rich list
- [ ] LLM 分支摘要：结构化输出 + token budget + 迭代更新
