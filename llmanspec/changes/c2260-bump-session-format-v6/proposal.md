---
depends_on: [c2250-harden-session-write-path]
---

# session 盘面 v6：unix-ms 时间戳统一 + 兼容 alias 清零

> **一句话**：盘面时间戳统一为 u64 unix 毫秒（壳 / header 与 message 内同基准），`SESSION_VERSION` 5→6（旧 v5 盘拒绝、不迁移），删除全部「pre-fix JSONL」serde alias；`streamTiming` 侧带补形状锁定测试。
> **目的地**：盘面单一时间基准 + 零兼容债；未发布阶段一步到位，公共版本前只剩「收紧侧带」一件事。

## Why

1. **两套时间戳基准**：条目壳与 header 是 RFC3339 字符串（`rfc3339_now`，`src/infra/session/manager.rs:15`），message 内是 u64 unix-ms（bridge DTO `now_ms`，`src/protocol/message.rs:19`）。`list_sessions` 解析 header RFC3339、失败回退文件 mtime（`manager.rs:1609-1620`）；排序 / 展示 / `streamTiming` 混用两套基准。
2. **代码违反自家 spec 的兼容债**：`agent-session-store` s18 明文「MUST NOT 提供 serde alias」，但 `EnvMessage` 9 个、bridge DTO 11 个 snake alias 的注释都写着「accept/read pre-fix JSONL」（`src/protocol/message.rs:30-87`、`packages/xylitol-ai-bridge/src/dto/message.rs` 多处）——正是根 AGENTS「Pre-0.0.1 卫生」禁止的未发布兼容层。alias 只服务读旧盘（上游 provider 原始键是 `cache_read_input_tokens` 类长名，与这些短 alias 无关），可全删。
3. **侧带零契约**：`thinkingElapsedSecs` / `streamTiming` 由 `persist_agent_message_with_thought_elapsed` 序列化后直注 JSON（`src/agent/runtime/react/support.rs:84-99`），读侧靠 serde 忽略未知字段 + TUI raw 索引（`src/app/tui/bridge/session_tree.rs`）。刻意设计（c2240），但改名 / 改形状无编译错误。公共版本前再收紧为 schema 字段；本票先锁形状防漂移。

## What Changes

1. **版本**：`SESSION_VERSION` 5→6（`src/protocol/session/entries.rs:12`），版本史注释补 v6 语义（= v5 + 壳/header 时间戳 u64 unix-ms + 无 alias）。`unsupported_session_version_msg` 随常量自动报告 require 6。
2. **时间戳统一**：`EntryBase.timestamp` 与 `SessionHeader.timestamp` 从 `String`（RFC3339）→ `u64`（unix ms）。
   - 写点：`inject_ids` / `clone_entry_with_ids`（`manager.rs:390,420`）、`create()` header（`:279`）改用 `now_ms`；`rfc3339_now` 无剩余调用则删。
   - 读点 / 消费点：`list_sessions` 删 RFC3339 解析、直接用 ms（mtime 回退保留，`manager.rs:1609-1620`）；HTML 导出标题在展示边格式化 ms→RFC3339（`src/app/core/session_export.rs:130`，用既有 `time` crate）；TUI resume 面已用 `modified_unix` 不变。执行时 `rg '\.timestamp' src/ src/app/ packages/` 扫尾。
3. **alias 清零**：删 `src/protocol/message.rs` 9 个（`exit_code` / `full_output_path` / `exclude_from_context` / `custom_type` / `tokens_before` / `tokens_after` / `read_files` / `modified_files` / `from_id`）+ bridge `dto/message.rs` 11 个（`stop_reason` / `response_id` / `error_message` / `tool_use_id` / `tool_name` / `is_error` / `cache_read` / `cache_write` / `cache_write_1h` + UsageCost 的 `cache_read` / `cache_write`），连同「aliases accept pre-fix JSONL」注释。跑全量测试确认 provider 适配器未依赖（BDD 会抓；若有断链改显式映射，不恢复 alias）。
4. **形状锁定测试**：persist 注入 → 读回 → 断言 `thinkingElapsedSecs`（u64）与 `streamTiming`（map<string,u64>）键存在且类型正确 + `AgentMessage` 类型化反序列化成功且忽略之。**不改变注入行为**（c2240 合约不变）。
5. **Specs landing**：s18 / s20 的 `SESSION_VERSION（5）`→`6`；新增盘面时间戳条款（确切文本见 design）。

## 非目标

- 旧 v5 盘迁移 / 双读：版本不符即快速报错（现有 `enforce_session_version` 行为，仅数字变化）；需要留存旧会话先用 `/session-export` 导出 JSONL（与存储同构，可作未来迁移源）
- `streamTiming` 升正式 schema 字段（公共版本前另票收紧）
- 文件名 / 目录布局（UUID 文件名与 mtime 排序保持）
- message 内 `timestamp` 字段（bridge DTO 契约，保留且已是 ms 基准）
- 跨进程锁 / 写入原子性（`c2250-harden-session-write-path`，本票 depends_on 它先行）

## Capabilities

- `agent-session-store`：s18 / s20 版本号更新 + 新增盘面时间戳条款
- 复核：执行时跑 `llman sdd context --task "session 盘面时间戳统一为 unix-ms、删除 serde alias、版本 6" --paths "src/protocol/session,src/protocol/message.rs,packages/xylitol-ai-bridge/src/dto/message.rs"`；候选波及 `domain-message`（dm1 AgentPart SSOT）与 `agent-session`（as45-as48 投影），确认无版本 / 时间戳硬编码再决定是否动

## Impact

- **盘面 v6 破坏性**：现有 `~/.xylitol/sessions` 的 v5 会话全部不可读（by design，报「require 6」可操作错误）；导出 / 导入面同步只认 v6
- 代码：`src/protocol/session/entries.rs`、`src/protocol/message.rs`、`packages/xylitol-ai-bridge/src/dto/message.rs`、`src/infra/session/manager.rs`、`src/app/core/session_export.rs` + 手写 RFC3339 的测试夹具（如 `session_export.rs:246,257`）
- BDD / feature 文件中若引用 version 5 或 RFC3339 盘面值一并更新

## Test seams

- entries / parse 单测：v6 round-trip；v5 文件被拒且错误文案含 `require 6`
- alias 删除回归：含 snake 字段的盘面行按缺失处理（Option→None / 默认值），不复活旧语义
- 形状锁定：`support.rs` persist 注入 → 读回断言两键存在且类型正确 + 类型化投影忽略
- `list_sessions`：ms 直读排序与 mtime 回退行为不变（仅换算路径变）

## Further Notes

- 调研证据（全部 file:line 与决策记录）：[`research/investigation.md`](./research/investigation.md)
- 前置票：`c2250-harden-session-write-path`（写入路径加固；本票 version bump 的落盘走其修好的路径）
