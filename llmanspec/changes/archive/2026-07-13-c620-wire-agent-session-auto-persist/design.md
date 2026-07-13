# Design — c620 session auto-persist（对齐 pi）

## 问题拆解

```text
TUI scrollback  ← XyEvent（内存）     ← 有字
session tree    ← store jsonl          ← 仅 header → (0/0)
Driver::run     → agent.run() → 每轮新 Uuid → 新文件
ReAct history   ← 本地 Vec，不 append store，也不从 store 灌回
```

## 决策

### 1. 稳定 session id

- `BootstrappedAgent::into_runtime`：`agent.set_session(session_id)` 后再包 `InProcessDriver`。
- `InProcessDriver::run`：`let sid = self.session_id().unwrap_or_else(|| mint_once); self.agent.run_with_id(prompt, &sid)`。
- 禁止生产路径每轮 `Uuid::new_v4()` 作为默认。

### 2. 消息 auto-persist（pi `message_end`）

在 ReAct 中于下列完成点调用 store append（`SessionEntry::Message`，message JSON = `AgentMessage` 序列化）：

| 时机 | 角色 |
|---|---|
| 本轮 user prompt 入 history 后 | user |
| assistant 流式结束、入 history 后 | assistant |
| tool 结果入 history 后 | toolResult |

与 model/thinking 变更的既有 append 并存。

### 3. 从 store 灌 history

`run_with_id`：`ensure_session` → `build_session_context` → `SessionEntry`→`AgentMessage`（复用 compaction `message_converter` / 等价）→ 作为初始 `history`，再 push 本轮 user。
travel / compact 后下一轮自然跟 leaf 路径一致（leaf 已由 c610 `travel_session_tree` 维护）。

### 4. 延迟落盘（pi `_persist`）

Persisted backend：

1. `create` / 早期 append（user 等）可只进内存挂起缓冲。
2. **首条 assistant** append 时：创建 `~/.xylitol/sessions/{id}.jsonl`（若无），按序写出 header + 挂起条目 + 本条。
3. 之后每次 append 直接 append 文件。
4. `load`：若文件存在读文件；否则读内存挂起。
5. `SessionManager::in_memory()`：永不创建文件。

允许与「create 立刻写 header」的旧行为迁移：以实现为准，测试锁定「仅 user、无 assistant 时不留下无消息垃圾文件」或「首条 assistant 后文件完整」二选一的可观测合约（delta s12）。

### 5. 单一 SessionManager

bootstrap / composition 保证 Agent 与 `InProcessDriver` 的 `store` 为同一 `Arc`（同一 leaf map）。禁止再各 `SessionManager::new(same_dir)` 一份。

## 非目标

- 不在本 change 引入 `--no-session` CLI。
- 不把 deferred 做成第三种用户可见模式；只是 Persisted 的写策略。
