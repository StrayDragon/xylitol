# 设计：session 盘面 v6

## 1. 版本语义

- `SESSION_VERSION = 6`。v6 = v5 全部语义 + 壳 / header 时间戳为 u64 unix-ms + 无任何 serde alias。
- 读写纪律不变：写 6、读拒绝非 6（`enforce_session_version`，`src/protocol/session/parse.rs:43`）、坏行 skip-warn（s20）。`peek_session_header_version`（`parse.rs:62`）与 `unsupported_session_version_msg`（`parse.rs:55`，用常量拼文案）随常量自动适配，无需改逻辑。
- 不迁移、不双读：现有 v5 盘的预期结局是「require 6」可操作错误。这是产品决定（Pre-0.0.1 无外部 SemVer 客户），不是遗漏。

## 2. 时间戳统一

### 基准决策

u64 unix-ms 为**唯一盘面基准**：与 message 内 `timestamp`（bridge DTO）、`streamTiming` 节点 ms、`now_ms()`（`src/protocol/message.rs:19`）全部同源。RFC3339 只在**展示边**格式化，不进存储。

理由（否决项）：全 RFC3339 需要动 bridge DTO 的 provider 面契约（`timestamp: u64` 是 DTO 字段），影响面大且字符串解析贵；ms 整数排序 / 比较零成本。

### 字段变更

`src/protocol/session/entries.rs`：

- `EntryBase.timestamp: String` → `u64`
- `SessionHeader.timestamp: String` → `u64`

### 写点

`src/infra/session/manager.rs`：

- `inject_ids` / `clone_entry_with_ids`（`:390,420` 一带）：`rfc3339_now()` → `now_ms()`（同一批条目共享一次取值，与现状共享一次 `rfc3339_now` 一致）
- `create()` header（`:279`）：同上
- `rfc3339_now`（`:15`）：扫尾无剩余调用即删除（Pre-0.0.1 卫生：不留死码）

### 读点 / 消费点

- `list_sessions`（`:1609-1620`）：删 `time::OffsetDateTime::parse(Rfc3339)` 分支，直接 `h.timestamp / 1000` → `modified_unix`（秒）。**语义维持现状**：header 时间戳（会话创建时刻）仍是列表时间源，文件 mtime 仅为回退——本票只换基准，不改「modified 用创建时刻」的现状语义（要改成 mtime 优先是独立产品决定）。
- HTML 导出标题（`src/app/core/session_export.rs:130`）：`@ {h.timestamp}` → 展示边格式化（`time` crate：`OffsetDateTime::from_unix_timestamp(ms/1000)` → Rfc3339 格式化；manager 已依赖 `time`）。
- TUI resume 面用 `SessionListEntry.modified_unix`（`src/app/tui/session_resume/panel.rs:334`），不感知变化。
- 扫尾指令：`rg '\.timestamp\b' src/ packages/xylitol-ai-bridge/src/`（排除 bridge DTO 自身字段定义与测试字面量），确认无其它壳时间戳消费者。

### 测试夹具

手写 RFC3339 字符串的夹具（`session_export.rs:246,257` 等）改 ms 字面量；`parse` / round-trip 测试同步。

## 3. alias 清零

### 清单（逐字段）

`src/protocol/message.rs`（EnvMessage，9）：

| 字段 | alias |
|---|---|
| BashExecutionMessage.exit_code | `exit_code` |
| BashExecutionMessage.full_output_path | `full_output_path` |
| BashExecutionMessage.exclude_from_context | `exclude_from_context` |
| CustomMessage.custom_type | `custom_type` |
| CompactionSummaryMessage.tokens_before | `tokens_before` |
| CompactionSummaryMessage.tokens_after | `tokens_after` |
| CompactionSummaryMessage.read_files | `read_files` |
| CompactionSummaryMessage.modified_files | `modified_files` |
| BranchSummaryMessage.from_id | `from_id` |

`packages/xylitol-ai-bridge/src/dto/message.rs`（11）：

| 字段 | alias |
|---|---|
| AssistantMessage.stop_reason | `stop_reason` |
| AssistantMessage.response_id | `response_id` |
| AssistantMessage.error_message | `error_message` |
| ToolResultMessage.tool_use_id | `tool_use_id` |
| ToolResultMessage.tool_name | `tool_name` |
| ToolResultMessage.is_error | `is_error` |
| AiBridgeUsage.cache_read | `cache_read` |
| AiBridgeUsage.cache_write | `cache_write` |
| AiBridgeUsage.cache_write_1h | `cache_write_1h` |
| AiBridgeUsageCost.cache_read | `cache_read` |
| AiBridgeUsageCost.cache_write | `cache_write` |

同步删除两文件中「snake aliases accept/read pre-fix JSONL」类注释（注释本身就在宣示兼容层）。

### 验证策略

- 依据：上游 provider 原始键与这些短 alias 无对应（Anthropic 是 `cache_read_input_tokens` / `cache_creation_input_tokens`，OpenAI 是 `cached_tokens`）；adapter 均显式映射到 DTO。预期删除断链为零。
- `just test` 全量（含 BDD）确认；若断链，**在 adapter 内显式映射修复，不恢复 alias**。
- 删除后的行为：v5 盘含 snake 字段的行 → Option 字段取 None、默认字段取默认（serde 忽略未知键）。注意这些 v5 行在 `enforce_session_version` 处已被整文件拒绝，不会走到字段层——字段层变化只影响 v6 手工构造数据。
- 保留（不是兼容，勿删）：parse skip-warn（s20 损坏容忍，也是 append 半行崩溃的兜底）；`merge_pending_ahead_of_disk`（并发防御）。

## 4. streamTiming 形状锁定（不改行为）

- 测试放 agent runtime persist 单测旁（`src/agent/runtime/react/` 既有 persist 测试处）。
- 断言三点：
  1. 带 clock 的 persist 后，message JSON 顶层存在 `thinkingElapsedSecs`（u64，有值时）与 `streamTiming`（`map<string,u64>`，只含发生过的节点）；
  2. `AgentMessage::deserialize` 对该 JSON 成功（未知字段忽略语义）；
  3. 类型化结果不含这两键的信息（投影不受污染）。
- c2240 合约不变：不升 schema 字段、不改注入位置。公共版本前的收紧（正式 schema 化）另票。

## 5. Specs landing 文本

`llmanspec/specs/agent-session-store/spec.toon`：

1. s18：`header.version MUST 等于 SESSION_VERSION（5）` → `（6）`；`MUST NOT 再写出 parent_id 或 version≤4 作为新会话真源` → `version≤5`。
2. s20：`header.version 不等于 SESSION_VERSION（5）` → `（6）`。
3. 新增（req_id 按文件现有编号习惯取下一个空位）：

```toon
s22,盘面时间戳,"Session 盘面时间戳（header.timestamp 与条目壳 timestamp）MUST 为 u64 unix 毫秒数，与 message 内 timestamp 同基准；MUST NOT 写出 RFC3339 字符串作为盘面时间戳。展示面需要人类可读时间时 MUST 在展示边格式化。"
```

4. scenarios 补一行非执行场景（feature=false）：给定 v6 会话文件，当读取 header 与条目壳，则 timestamp 均为 unix-ms 整数。
5. `llman sdd context` 复核 `domain-message` / `agent-session` 是否有版本或时间戳硬编码条款；有则同步，无则不动。

## 6. 风险与回滚

- 主风险是**漏改消费者**（壳时间戳被某处当字符串用）→ 编译期即暴露（类型 String→u64），rust-analyzer 全量可跳转，属低风险。
- BDD feature 文件若硬编码 version / RFC3339 值会红 → 属预期更新范围。
- 无运行时回滚需求：盘面 v6 一旦写入即新真源；旧 v5 盘保持原样（不被读取、不被改写）。
