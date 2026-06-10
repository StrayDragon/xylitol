# Design: Phase 2 Core Gaps

## 设计原则

1. **最小化侵入** — 尽量在现有代码上增量修改，避免大规模重写
2. **与 pi 架构对齐** — 每个模块的 API 设计与 pi TypeScript 尽量一致，便于对照
3. **向后兼容** — AgentEventStream → EventBus 迁移提供 Stream 适配器

---

## 1. 会话树形结构

### 当前状态
SessionEntry 是扁平枚举，无 `id`/`parent_id`。SessionManager 无树导航能力。

### 设计方案

**Entry 升级**:

```rust
struct EntryBase {
    entry_type: String,     // "message", "compaction", ...
    id: String,             // NEW: UUID v4
    parent_id: Option<String>, // NEW: 父条目 ID
    timestamp: String,
}

enum SessionEntry {
    Message { base: EntryBase, message: Value },
    Compaction { base: EntryBase, summary, first_kept_entry_id, tokens_before, details },
    BranchSummary { base: EntryBase, from_id, summary, details, from_hook },
    ModelChange { base: EntryBase, provider, model_id },   // NEW
    ThinkingLevelChange { base: EntryBase, level },         // NEW
    CustomMessage { base: EntryBase, custom_type, content, display, details }, // NEW
}
```

**SessionManager 新增字段**:

```rust
struct SessionManager {
    sessions_dir: PathBuf,
    // NEW:
    current_leaf_id: Option<String>,  // 写入时自动维护
}
```

**append() 流程**:
1. 生成 UUID v4 作为 entry.id
2. parent_id = current_leaf_id（上一个条目的 id）
3. current_leaf_id = entry.id
4. 写入 JSONL

**buildSessionContext() 流程**:
1. load all entries
2. 建立 id→entry 的 HashMap
3. 从 current_leaf_id 开始，沿 parent_id 链走到 root (parent_id==null)
4. 将沿途的消息条目按类型转换为 XyContent
5. 反向排序（root→leaf）返回 Vec<XyContent>

**v3→v4 迁移**:
- load 时检测 header.version < 4
- 为每个条目生成 UUID，建立链（上一个条目的 id 作为下一个的 parent_id）
- 更新 header.version = 4

### 与 pi 的对应关系

| pi | xylitol |
|----|---------|
| `entry.id`, `entry.parentId` | `EntryBase.id`, `EntryBase.parent_id` |
| `getLeafId()`, `getLeafEntry()` | `current_leaf_id` 字段 |
| `getBranch(leafId)` | `get_branch()` |
| `buildSessionContext()` | `build_session_context()` |
| `migrateV1ToV2`, `migrateV2ToV3` | v3→v4 迁移 |

---

## 2. 事件订阅模型

### 方案: `tokio::sync::broadcast`

```rust
use tokio::sync::broadcast;

pub struct AgentEventBus {
    sender: broadcast::Sender<AgentEvent>,
}

impl AgentEventBus {
    pub fn new(capacity: usize) -> Self { ... }
    pub fn subscribe(&self) -> UnsubscribeHandle { ... }
    pub fn emit(&self, event: AgentEvent) { ... }
}

pub struct UnsubscribeHandle {
    receiver: broadcast::Receiver<AgentEvent>,
}

impl Drop for UnsubscribeHandle {
    // drop receiver → auto-unsubscribe
}
```

**AgentLoop 集成**:
- `AgentLoop::run()` 内部创建 EventBus，每个 turn 通过 `emit()` 发送事件
- 返回 `(UnsubscribeHandle, JoinHandle<()>)` — handle 可订阅事件，JoinHandle 等待循环完成
- 提供 `AgentEventStream::from_bus(bus)` 适配器用于向后兼容

---

## 3. 自动压缩集成

### 流程

```
agent_end 事件 → 检查 compaction 条件:
  1. overflow 检测: stop_reason == "error" && "context length" in message
     → _check_overflow()
  2. threshold 检测: estimate_tokens > context_window * threshold
     → _check_threshold()

_check_overflow():
  1. 从状态中移除错误 assistant 消息
  2. 调用 compact_session()
  3. 重新发送相同 prompt（retry once）
  4. 如果再次 overflow → 发出 compaction_end 错误，停止

_check_threshold():
  1. 调用 compact_session()
  2. 返回 true（调用者继续循环）
```

### 与 Agent 循环的集成点

在 `run_react_loop` 中，assistant 响应后，发送 `agent_end` 事件**之前**检查 compaction。

---

## 4. 自动重试

### 错误模式匹配

```rust
fn is_retryable_error(error_msg: &str) -> bool {
    let retryable = Regex::new(
        r"(?i)overloaded|rate.limit|too many requests|429|500|502|503|504|
           service.unavailable|server.error|internal.error|network.error|
           connection.refused|connection.lost|timeout|terminated"
    ).unwrap();

    let non_retryable = Regex::new(
        r"(?i)usage_limit|insufficient_quota|billing"
    ).unwrap();

    !non_retryable.is_match(error_msg) && retryable.is_match(error_msg)
}
```

### RetryState

```rust
struct RetryState {
    max_retries: u32,          // default 3
    base_delay_ms: u64,        // default 1000
    attempt_count: u32,
    abort: watch::Sender<bool>,
}
```

### 指数退避

delay = base_delay_ms * 2^(attempt-1)：1s, 2s, 4s, 8s...

---

## 5. 队列管理

### MessageQueue

```rust
struct MessageQueue {
    steering: VecDeque<(String, Vec<XyPart>)>,  // 打断消息
    follow_up: VecDeque<(String, Vec<XyPart>)>,  // 跟进消息
    next_turn: VecDeque<XyContent>,              // 下轮注入
}
```

### 交付时机

- **steer**: 当前 tool calls 全部执行完毕后，下次 LLM 调用前
- **followUp**: agent 无 pending operations 时
- **next_turn**: 下次用户 prompt 时，作为前置消息

---

## 6. 系统提示词

### build_system_prompt 函数

```rust
struct SystemPromptOpts {
    custom_prompt: Option<String>,
    selected_tools: Vec<String>,
    tool_snippets: HashMap<String, String>,
    tool_guidelines: Vec<String>,
    append_prompt: Option<String>,
    cwd: String,
    skills: Vec<Skill>,
    context_files: Vec<ContextFile>,
    date: String,
}
```

输出结构：
```
You are an expert coding assistant...

Available tools:
- read: ...
- write: ...

Guidelines:
- ...

<project_context>
...
</project_context>

<skills>
...
</skills>

Current date: ...
Current working directory: ...
```

---

## 7. LLM 分支摘要

复用 c08 建立的 compaction 提示词基础设施，新增：

```rust
const BRANCH_SUMMARY_PROMPT: &str = "
The user explored a different conversation branch before returning here.
Summary of that exploration:

## Goal
...

## Progress
### Done
...

## Next Steps
...";
```

**Token Budget**: context_window - reserve_tokens (默认 16384)，从最新消息走到最旧，直到预算耗尽。

**迭代更新**: 当路径中包含已有的 compaction/branch_summary 条目时，使用 `UPDATE_SUMMARIZATION_PROMPT` 增量更新而不是从头生成。

**Fallback**: 当 model 不可用（如无 API key），退化为纯文本统计（当前 c15 实现）。
